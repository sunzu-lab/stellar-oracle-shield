#![cfg(test)]

use {
    super::*,
    soroban_sdk::testutils::{
        Address as AddressTrait, AuthorizedFunction, AuthorizedInvocation, Events,
    },
    soroban_sdk::{events::Event, vec, Env, IntoVal, Symbol, Val, Vec},
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
    let constructor_args = (&admin_address,);
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
    let constructor_args = (&admin_address,);
    let contract_id = env.register(Contract, constructor_args.clone());

    let client = ContractClient::new(&env, &contract_id);
    let base = usdc_circle_address(&env);
    let quote = xlm_address(&env);

    client.set_score(&base, &quote, &12_i32);
    assert_eq!(
        env.auths(),
        [contract_auth_for(
            &env,
            admin_address.clone(),
            contract_id.clone(),
            "set_score",
            (base.clone(), quote.clone(), 12_i32,)
        )]
    );
    assert_eq!(client.get_score(&base, &quote), 12);

    let ret = client.try_set_score(&base, &quote, &12345678_i32);
    assert_eq!(ret, Err(Ok(Error::ScoreBounds)));

    let ret = client.try_set_score(&base, &quote, &-2_i32);
    assert_eq!(ret, Err(Ok(Error::ScoreBounds)));
}

#[test]
fn test_get_score() {
    let env = Env::default();
    env.mock_all_auths();

    let admin_address = Address::generate(&env);
    let constructor_args = (&admin_address,);
    let contract_id = env.register(Contract, constructor_args.clone());

    let client = ContractClient::new(&env, &contract_id);

    let base = usdc_circle_address(&env);
    let quote = xlm_address(&env);
    assert_eq!(
        client.try_get_score(&base, &quote),
        Err(Ok(Error::PairNotCovered))
    );
    assert!(env.auths().is_empty());

    client.set_score(&base, &quote, &12_i32);

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

    let admin_address = Address::generate(&env);
    let constructor_args = (&admin_address,);
    let contract_id = env.register(Contract, constructor_args.clone());

    let client = ContractClient::new(&env, &contract_id);

    let base = usdc_circle_address(&env);
    let quote = xlm_address(&env);
    assert_eq!(
        client.try_get_status(&base, &quote),
        Err(Ok(Error::PairNotCovered))
    );
    assert!(env.auths().is_empty());

    client.set_score(&base, &quote, &0_i32);
    assert_eq!(client.get_status(&base, &quote), Status::Unsafe);

    client.set_score(&base, &quote, &32_i32);
    assert_eq!(client.get_status(&base, &quote), Status::Unsafe);

    client.set_score(&base, &quote, &33_i32);
    assert_eq!(client.get_status(&base, &quote), Status::Degraded);

    client.set_score(&base, &quote, &65_i32);
    assert_eq!(client.get_status(&base, &quote), Status::Degraded);

    client.set_score(&base, &quote, &66_i32);
    assert_eq!(client.get_status(&base, &quote), Status::Healthy);

    client.set_score(&base, &quote, &100_i32);
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

    let admin_address = Address::generate(&env);
    let constructor_args = (&admin_address,);
    let contract_id = env.register(Contract, constructor_args.clone());

    let client = ContractClient::new(&env, &contract_id);

    let base = usdc_circle_address(&env);
    let quote = xlm_address(&env);

    client.set_score(&base, &quote, &0_i32);
    assert!(env
        .events()
        .all()
        .filter_by_contract(&contract_id)
        .events()
        .is_empty());

    client.set_score(&base, &quote, &10_i32);
    assert!(env
        .events()
        .all()
        .filter_by_contract(&contract_id)
        .events()
        .is_empty());

    client.set_score(&base, &quote, &44_i32);
    let event = env.events().all().filter_by_contract(&contract_id);
    let expected = StatusChange {
        base: base.clone(),
        quote: quote.clone(),
        status: Status::Degraded,
    };

    assert_eq!(event.events(), &[expected.to_xdr(&env, &contract_id)]);

    client.set_score(&base, &quote, &66_i32);
    let event = env.events().all();

    assert_eq!(
        event,
        vec![
            &env,
            (
                contract_id,
                (Symbol::new(&env, "HEALTH_CHANGED"), base, quote).into_val(&env),
                Status::Healthy.into_val(&env)
            )
        ]
    )
}
