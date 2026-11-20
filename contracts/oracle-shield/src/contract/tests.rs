#![cfg(test)]

extern crate std;

use {
    super::*,
    soroban_sdk::testutils::{
        Address as AddressTrait, AuthorizedFunction, AuthorizedInvocation, ContractEvents, Events,
        Ledger,
    },
    soroban_sdk::{
        Env, IntoVal, Symbol, TryFromVal, Val, Vec,
        events::Event,
        vec,
        xdr::{ContractEvent, ContractEventBody, ScVal},
    },
    std::string::ToString,
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

fn test_default_parameters(env: &Env) -> Parameters {
    let operator_address = Address::generate(&env);
    let max_staleness = 60_u64;
    let max_deviation: u32 = 5;
    let quorum: u32 = 1;
    let operators = Map::from_array(env, [(operator_address.clone(), ())]);
    Parameters {
        max_staleness,
        max_deviation,
        quorum,
        operators,
    }
}

fn test_default_governance_parameters() -> GovernanceParameters {
    let timelock_delay_seconds = 6_u64;
    let execution_window_seconds = 86_400_u64;
    GovernanceParameters {
        timelock_delay_seconds,
        execution_window_seconds,
    }
}

fn test_default_governamce_parameters_constraints() -> GovernanceParametersConstraints {
    let min_timelock_delay_seconds = 3_u64;
    let max_timelock_delay_seconds = 12_u64;

    GovernanceParametersConstraints {
        min_timelock_delay_seconds,
        max_timelock_delay_seconds,
    }
}

fn build_contract_client(env: &Env) -> (ContractClient<'_>, Address, Address, Address) {
    let admin_address = Address::generate(&env);
    let parameters = Some(test_default_parameters(env));
    let governance_parameters = Some(test_default_governance_parameters());
    let governance_parameters_constraints = Some(test_default_governamce_parameters_constraints());
    build_contract_client_with_parameters(
        env,
        admin_address,
        parameters,
        governance_parameters,
        governance_parameters_constraints,
    )
}

fn build_contract_client_with_parameters(
    env: &Env,
    admin_address: Address,
    parameters: Option<Parameters>,
    governance_parameters: Option<GovernanceParameters>,
    governance_parameters_constraints: Option<GovernanceParametersConstraints>,
) -> (ContractClient<'_>, Address, Address, Address) {
    let operator_address = parameters
        .as_ref()
        .unwrap()
        .operators
        .keys()
        .first()
        .unwrap();
    let constructor_args = (
        admin_address.clone(),
        parameters,
        governance_parameters,
        governance_parameters_constraints,
    );
    let contract_id = env.register(Contract, constructor_args);

    (
        ContractClient::new(&env, &contract_id),
        contract_id,
        operator_address,
        admin_address,
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

/// helpers on contract
trait WaitTimelock {
    /// ensure execute_xxx won't trigger a TimedLocked error
    fn _wait_timelock(self: &Self);

    /// direct max staleness setter (bypass time locked schedule_set_max_staleness)
    fn _set_max_staleness(self: &Self, max_staleness: u64);

    /// direct quorum setter (bypass time locked schedule_set_quorum)
    fn _set_quorum(self: &Self, quorum: u32);

    /// direct operators setter (bypass time locked schedule_add_operator)
    fn _add_operator(self: &Self, operator: Address);

    /// direct operators setter (bypass time locked schedule_remove_operator)
    fn _remove_operator(self: &Self, operator: Address);
}

impl<'a> WaitTimelock for ContractClient<'a> {
    fn _wait_timelock(self: &Self) {
        let timelock_delay_seconds = self.get_governance_params().timelock_delay_seconds;
        self.env
            .ledger()
            .set_timestamp(self.env.ledger().timestamp() + timelock_delay_seconds);
    }

    fn _set_max_staleness(self: &Self, max_staleness: u64) {
        let mut parameters = self.get_parameters();
        parameters.max_staleness = max_staleness;
        self._set_parameters(parameters);
    }

    fn _set_quorum(self: &Self, quorum: u32) {
        let mut parameters = self.get_parameters();
        parameters.quorum = quorum;
        self._set_parameters(parameters);
    }

    fn _add_operator(self: &Self, operator: Address) {
        let mut parameters = self.get_parameters();
        parameters.operators.remove(operator);
        self._set_parameters(parameters);
    }

    fn _remove_operator(self: &Self, operator: Address) {
        let mut parameters = self.get_parameters();
        parameters.operators.remove(operator);
        self._set_parameters(parameters);
    }
}

/// give access to internal methods for testing purpose
trait ContractPrivateMethods {
    fn _get_inner_score(
        self: &Self,
        base: &Address,
        quote: &Address,
    ) -> Result<AggregatedScore, Error>;
    fn _set_parameters(self: &Self, parameters: Parameters);
}

impl<'a> ContractPrivateMethods for ContractClient<'a> {
    fn _get_inner_score(
        self: &Self,
        base: &Address,
        quote: &Address,
    ) -> Result<AggregatedScore, Error> {
        let body = || Contract::get_inner_score(&self.env, &Pair(base.clone(), quote.clone()));
        self.env.as_contract(&self.address, body)
    }

    fn _set_parameters(self: &Self, parameters: Parameters) {
        let body = || {
            Contract::set_parameters(&self.env, parameters);
        };
        self.env.as_contract(&self.address, body)
    }
}

fn get_topics(env: &Env, contract_event: &ContractEvent) -> std::vec::Vec<std::string::String> {
    let scval_to_symbol = |s: &ScVal| -> std::string::String {
        match s {
            ScVal::Symbol(s) => Symbol::try_from_val(env, s).unwrap().to_string(),
            _ => std::string::String::default(),
        }
    };

    match &contract_event.body {
        ContractEventBody::V0(body) => body.topics.iter().map(scval_to_symbol).collect(),
    }
}

fn get_all_topics(env: &Env, events: &ContractEvents) -> std::vec::Vec<std::string::String> {
    events
        .events()
        .iter()
        .flat_map(|e| get_topics(env, e))
        .collect()
}

#[test]
fn test_constructor() {
    let env = Env::default();
    env.mock_all_auths();

    let admin_address = Address::generate(&env);
    let parameters = Some(test_default_parameters(&env));
    let governance_parameters = Some(test_default_governance_parameters());
    let governance_parameters_constraints = Some(test_default_governamce_parameters_constraints());
    let constructor_args = (
        admin_address.clone(),
        parameters,
        governance_parameters,
        governance_parameters_constraints,
    );
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
#[should_panic(expected = "constructor invocation has failed")]
fn test_constructor_invalid_governance() {
    let env = Env::default();
    env.mock_all_auths();

    let admin_address = Address::generate(&env);
    let mut parameters = test_default_parameters(&env);
    let governance_parameters = Some(test_default_governance_parameters());
    let governance_parameters_constraints = Some(test_default_governamce_parameters_constraints());

    // constructor - invalid parameters
    parameters.quorum = 64_u32;
    let parameters = Some(parameters);

    let _ = build_contract_client_with_parameters(
        &env,
        admin_address,
        parameters,
        governance_parameters,
        governance_parameters_constraints,
    );
}

#[test]
#[should_panic(expected = "constructor invocation has failed")]
fn test_constructor_invalid_parameters() {
    let env = Env::default();
    env.mock_all_auths();

    let admin_address = Address::generate(&env);
    let parameters = Some(test_default_parameters(&env));

    let min_timelock_delay_seconds = 64_u64;
    let max_timelock_delay_seconds = 128_u64;

    let governance_parameters_constraints = Some(GovernanceParametersConstraints {
        min_timelock_delay_seconds,
        max_timelock_delay_seconds,
    });

    // constructor - timelock out of constraint range
    let timelock_delay_seconds = 0_u64;
    let execution_window_seconds = 86_400_u64;
    let governance_parameters = Some(GovernanceParameters {
        timelock_delay_seconds,
        execution_window_seconds,
    });

    let _ = build_contract_client_with_parameters(
        &env,
        admin_address,
        parameters,
        governance_parameters,
        governance_parameters_constraints,
    );
}

#[test]
#[should_panic(expected = "constructor invocation has failed")]
fn test_constructor_invalid_governance_constraints() {
    let env = Env::default();
    env.mock_all_auths();

    let admin_address = Address::generate(&env);
    let parameters = Some(test_default_parameters(&env));
    let governance_parameters = Some(test_default_governance_parameters());

    // constructor - invalid range
    let min_timelock_delay_seconds = 64_u64;
    let max_timelock_delay_seconds = 0_u64;

    let governance_parameters_constraints = Some(GovernanceParametersConstraints {
        min_timelock_delay_seconds,
        max_timelock_delay_seconds,
    });

    let _ = build_contract_client_with_parameters(
        &env,
        admin_address,
        parameters,
        governance_parameters,
        governance_parameters_constraints,
    );
}

mod client;
mod governance;
mod quorum;
mod scoring;
