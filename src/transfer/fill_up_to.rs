use anyhow::{Context as _, Result};
use futures::future::join_all;
use solana_account_decoder::UiDataSliceConfig;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_rpc_client_api::config::RpcAccountInfoConfig;
use solana_sdk::{
    account::Account, native_token::Sol, pubkey::Pubkey, signature::Keypair, signer::Signer as _,
    system_instruction, transaction::Transaction,
};

use crate::{
    args::{json_rpc_url_args::get_rpc_client, transfer::fill_up_to::FillUpToArgs},
    blockhash_cache::BlockhashCache,
    keypair_ext::read_keypair_file,
    tx_sheppard::with_sheppard,
};

pub async fn run(
    FillUpToArgs {
        json_rpc_url,
        signer_keypair,
        payer_keypair,
        from_keypair,
        target_balance,
        print_target_increments,
        recepients,
    }: FillUpToArgs,
) -> Result<()> {
    let rpc_client = get_rpc_client(json_rpc_url);
    let rpc_client = &rpc_client;

    let signer = read_keypair_file(&signer_keypair)?;

    let payer = payer_keypair.map(read_keypair_file).transpose()?;
    let payer = payer.as_ref().unwrap_or(&signer);
    let payer_pubkey = payer.pubkey();

    let from = from_keypair.map(read_keypair_file).transpose()?;
    let from = from.as_ref().unwrap_or(payer);
    let from_pubkey = from.pubkey();

    let actions = join_all(
        recepients
            .into_iter()
            .map(|recepient| calculate_account_action(rpc_client, recepient, target_balance)),
    )
    .await
    .into_iter()
    .filter(|action_or_err| {
        // Keep errors.
        let Ok(AccountAction { add_lamports, .. }) = action_or_err else {
            return true;
        };

        // But skip any accounts that have enough already.
        *add_lamports > 0
    })
    .collect::<Result<Vec<_>>>()?;

    if print_target_increments {
        print_account_actions(&actions);
    }

    let minimum_balance = actions
        .iter()
        .map(|AccountAction { add_lamports, .. }| *add_lamports)
        .sum::<u64>();
    if !from_account_has_enough_balance(rpc_client, from_pubkey, minimum_balance).await? {
        return Ok(());
    }

    with_sheppard(rpc_client)
        .run(
            actions
                .iter()
                .map(|action| fill_up_tx(&signer, payer, payer_pubkey, from, from_pubkey, action)),
        )
        .await
        .with_context(|| "Running transfer transactions".to_owned())?;

    Ok(())
}

struct AccountAction {
    recepient: Pubkey,
    create: bool,
    add_lamports: u64,
}

async fn calculate_account_action(
    rpc_client: &RpcClient,
    recepient: Pubkey,
    target_balance: u64,
) -> Result<AccountAction> {
    // TODO It would be more efficient to use `get_multiple_accounts_with_config()`.  Note that it
    // requires pagination, as the method can query only up to 100 addresses per request.
    let account = rpc_client
        .get_account_with_config(
            &recepient,
            RpcAccountInfoConfig {
                data_slice: Some(UiDataSliceConfig {
                    offset: 0,
                    length: 0,
                }),
                ..RpcAccountInfoConfig::default()
            },
        )
        .await
        .with_context(|| format!("Reading account data for {recepient}"))?
        .value;

    let Some(Account { lamports, .. }) = account else {
        return Ok(AccountAction {
            recepient,
            create: true,
            add_lamports: target_balance,
        });
    };

    Ok(AccountAction {
        recepient,
        create: false,
        add_lamports: target_balance.saturating_sub(lamports),
    })
}

fn print_account_actions(actions: &[AccountAction]) {
    for AccountAction {
        recepient,
        create,
        add_lamports,
    } in actions
    {
        if !create {
            eprintln!(
                "Increasing {} balance by {} ...",
                recepient,
                Sol(*add_lamports)
            );
        } else {
            eprintln!(
                "Creating {} with a balance of {} ...",
                recepient,
                Sol(*add_lamports)
            );
        }
    }
}

async fn from_account_has_enough_balance(
    rpc_client: &RpcClient,
    from: Pubkey,
    minimum_balance: u64,
) -> Result<bool> {
    let account = rpc_client
        .get_account_with_config(
            &from,
            RpcAccountInfoConfig {
                data_slice: Some(UiDataSliceConfig {
                    offset: 0,
                    length: 0,
                }),
                ..RpcAccountInfoConfig::default()
            },
        )
        .await
        .with_context(|| format!("Reading account data for {from}"))?
        .value;

    let Some(Account { lamports, .. }) = account else {
        eprintln!("From account ({from}) does not exist");
        return Ok(false);
    };

    if lamports < minimum_balance {
        eprintln!(
            "From account ({}) balance is below the required minimum balance.\n\
             Current balance: {}\n\
             Minimum required to cover all the recipients: {}",
            from,
            Sol(lamports),
            Sol(minimum_balance),
        );
        return Ok(false);
    }

    Ok(true)
}

fn fill_up_tx<'context>(
    signer: &'context Keypair,
    payer: &'context Keypair,
    payer_pubkey: Pubkey,
    from: &'context Keypair,
    from_pubkey: Pubkey,
    AccountAction {
        recepient,
        create: _,
        add_lamports,
    }: &'context AccountAction,
) -> impl Fn(/* blockhash_cache: */ &BlockhashCache) -> Transaction + 'context {
    move |blockhash_cache: &BlockhashCache| -> Transaction {
        assert!(
            *add_lamports > 0,
            "`add_lamports` must be strictly positive when constructing a fill up transaction"
        );

        Transaction::new_signed_with_payer(
            &[system_instruction::transfer(
                &from_pubkey,
                recepient,
                *add_lamports,
            )],
            Some(&payer_pubkey),
            &[&signer, &payer, &from],
            blockhash_cache.get(),
        )
        // }
    }
}
