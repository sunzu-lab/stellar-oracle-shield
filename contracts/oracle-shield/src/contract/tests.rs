#![cfg(test)]

use {
    super::*,
    soroban_sdk::testutils::{
        Address as AddressTrait, AuthorizedFunction, AuthorizedInvocation, Events, Ledger,
    },
    soroban_sdk::{Env, IntoVal, Symbol, Val, Vec, events::Event, vec},
};

fn contract_auth_for(
    env: &Env,
    address: Address,
    contract: Address,
    function: &str,
    args: impl IntoVal<Env, Vec<Val>>,
) -> (Address, AuthorizedInvocation) {
    (
        address.clone(),
        AuthorizedInvocation {
            function: AuthorizedFunction::Contract((
                contract,
                Symbol::new(env, function),
                args.into_val(env),
            )),
            sub_invocations: [].into(),
        },
    )
}

fn c_client(env: &Env) -> ContractClient<'_> {
    let admin_address = Address::generate(&env);
    let constructor_args = (&admin_address, 60_u64);
    let contract_id = env.register(Contract, constructor_args.clone());

    ContractClient::new(&env, &contract_id)
}

fn xlm_address(env: &Env) -> Address {
    Address::from_str(
        env,
        "CAS3J7GYLGXMF6TDJBBYYSE3HQ6BBSMLNUQ34T6TZMYMW2EVH34XOWMA",
    )
}

fn usdc_circle_address(env: &Env) -> Address {
    Address::from_str(
        env,
        "CCW67TSZV3SSS2HXMBQ5JFGCKJNXKZM7UQUWUZPUTHXSTZLEO7SJMI75",
    )
}

#[test]
fn test_constructor() {
    let env = Env::default();
    env.mock_all_auths();

    let admin_address = Address::generate(&env);
    let constructor_args = (&admin_address, 60_u64);
    let contract_id = env.register(Contract, constructor_args.clone());
    assert_eq!(
        env.auths(),
        [contract_auth_for(
            &env,
            admin_address.clone(),
            contract_id.clone(),
            "__constructor",
            constructor_args
        )]
    );
}

#[test]
fn test_set_score() {
    let env = Env::default();
    env.mock_all_auths();

    let admin_address = Address::generate(&env);
    let constructor_args = (&admin_address, 60_u64);

    let contract_id = env.register(Contract, constructor_args.clone());

    let client = ContractClient::new(&env, &contract_id);
    let base = usdc_circle_address(&env);
    let quote = xlm_address(&env);

    client.set_score(&base, &quote, &12_u32);
    assert_eq!(
        env.auths(),
        [contract_auth_for(
            &env,
            admin_address.clone(),
            contract_id.clone(),
            "set_score",
            (base.clone(), quote.clone(), 12_u32,)
        )]
    );
    assert_eq!(client.get_score(&base, &quote), 12);

    let ret = client.try_set_score(&base, &quote, &12345678_u32);
    assert_eq!(ret, Err(Ok(Error::ScoreBounds)));
}

#[test]
fn test_get_score() {
    let env = Env::default();
    env.mock_all_auths();

    let client = c_client(&env);

    let base = usdc_circle_address(&env);
    let quote = xlm_address(&env);
    assert_eq!(
        client.try_get_score(&base, &quote),
        Err(Ok(Error::PairNotCovered))
    );
    assert!(env.auths().is_empty());

    client.set_score(&base, &quote, &12_u32);

    assert_eq!(client.get_score(&base, &quote), 12);
    assert!(env.auths().is_empty());

    assert_eq!(
        client.try_get_score(&quote, &quote),
        Err(Ok(Error::PairNotCovered))
    );
}

#[test]
fn test_get_status() {
    let env = Env::default();
    env.mock_all_auths();

    let client = c_client(&env);

    let base = usdc_circle_address(&env);
    let quote = xlm_address(&env);
    assert_eq!(
        client.try_get_status(&base, &quote),
        Err(Ok(Error::PairNotCovered))
    );
    assert!(env.auths().is_empty());

    client.set_score(&base, &quote, &0_u32);
    assert_eq!(client.get_status(&base, &quote), Status::Unsafe);

    client.set_score(&base, &quote, &32_u32);
    assert_eq!(client.get_status(&base, &quote), Status::Unsafe);

    client.set_score(&base, &quote, &33_u32);
    assert_eq!(client.get_status(&base, &quote), Status::Degraded);

    client.set_score(&base, &quote, &65_u32);
    assert_eq!(client.get_status(&base, &quote), Status::Degraded);

    client.set_score(&base, &quote, &66_u32);
    assert_eq!(client.get_status(&base, &quote), Status::Healthy);

    client.set_score(&base, &quote, &100_u32);
    assert_eq!(client.get_status(&base, &quote), Status::Healthy);

    assert_eq!(
        client.try_get_status(&quote, &quote),
        Err(Ok(Error::PairNotCovered))
    );
}

#[test]
fn test_event() {
    let env = Env::default();
    env.mock_all_auths();

    let client = c_client(&env);
    let contract_id = client.address.clone();

    let base = usdc_circle_address(&env);
    let quote = xlm_address(&env);

    client.set_score(&base, &quote, &0_u32);
    assert!(
        env.events()
            .all()
            .filter_by_contract(&contract_id)
            .events()
            .is_empty()
    );

    client.set_score(&base, &quote, &10_u32);
    assert!(
        env.events()
            .all()
            .filter_by_contract(&contract_id)
            .events()
            .is_empty()
    );

    client.set_score(&base, &quote, &44_u32);
    let event = env.events().all().filter_by_contract(&contract_id);
    let expected = StatusChange {
        base: base.clone(),
        quote: quote.clone(),
        status: Status::Degraded,
    };

    assert_eq!(event.events(), &[expected.to_xdr(&env, &contract_id)]);

    client.set_score(&base, &quote, &66_u32);
    let event = env.events().all();

    assert_eq!(
        event,
        vec![
            &env,
            (
                contract_id,
                (Symbol::new(&env, "status_change"), base, quote).into_val(&env),
                Status::Healthy.into_val(&env)
            )
        ]
    )
}

#[test]
fn test_max_staleness() {
    let env = Env::default();
    env.mock_all_auths();

    let client = c_client(&env);
    let contract_id = client.address.clone();

    let val: Option<u64> = env.as_contract(&contract_id, || {
        env.storage().instance().get(&DataKey::MaxStaleness)
    });
    assert_eq!(val, Some(60_u64));

    client.set_max_staleness(&300_u64);
    let val: Option<u64> = env.as_contract(&contract_id, || {
        env.storage().instance().get(&DataKey::MaxStaleness)
    });
    assert_eq!(val, Some(300_u64));

    let base = usdc_circle_address(&env);
    let quote = xlm_address(&env);

    client.set_score(&base, &quote, &12_u32);
    assert_eq!(client.get_score(&base, &quote), 12);

    env.ledger().with_mut(|ledger| {
        ledger.timestamp += 100;
    });
    assert_eq!(client.get_score(&base, &quote), 12);

    env.ledger().with_mut(|ledger| {
        ledger.timestamp += 201;
    });
    assert_eq!(
        client.try_get_score(&base, &quote),
        Err(Ok(Error::StaleInput))
    );

    env.ledger().with_mut(|ledger| {
        ledger.timestamp += 2;
    });
    client.set_score(&base, &quote, &12_u32);
    assert_eq!(client.get_score(&base, &quote), 12);
}
