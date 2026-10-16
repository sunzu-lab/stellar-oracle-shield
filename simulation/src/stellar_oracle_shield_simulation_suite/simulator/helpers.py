from stellar_sdk import scval, xdr
from stellar_sdk.sep.contract_spec import ContractSpec

from contextlib import contextmanager
from collections.abc import Iterator, Sequence, Mapping
from dataclasses import dataclass

from stellar_oracle_shield_client.bindings import Error as OracleShieldError
from stellar_sdk.contract.exceptions import (
    SimulationFailedError,
    NeedsMoreSignaturesError,
)

import re


def build_constructor_args(
    spec: ContractSpec,
    supplied: Mapping[str, xdr.SCVal],
) -> list[xdr.SCVal]:
    constructor = spec.get_function("__constructor")

    if constructor is None:
        if supplied:
            raise ValueError("Contract has no constructor")
        return []

    names = {parameter.name.decode("utf-8") for parameter in constructor.inputs}

    unknown = supplied.keys() - names
    if unknown:
        raise ValueError(f"Unknown constructor arguments: {sorted(unknown)}")

    args: list[xdr.SCVal] = []

    for parameter in constructor.inputs:
        name = parameter.name.decode("utf-8")

        if name in supplied:
            args.append(supplied[name])
        elif parameter.type.type == xdr.SCSpecType.SC_SPEC_TYPE_OPTION:
            args.append(scval.to_void())
        else:
            raise ValueError(f"Missing required constructor argument: {name}")

    return args


@dataclass
class ContractErrorResult:
    error: OracleShieldError | str | None = None


@contextmanager
def expect_contract_error() -> Iterator[ContractErrorResult]:
    """
    context returning OracleShieldError if one was raise, str if another error was raise, None if otherwise
    usage:
    ```
    with expect_contract_error() as error_returned:
        # tested code
    ```
    """
    result = ContractErrorResult()

    try:
        yield result
    except SimulationFailedError as scx:
        message = str(scx)
        result.error = message  # Fallback if parsing fails

        # Inspect the primary error, excluding diagnostic event logs.
        primary_error = message.split("Event log", 1)[0]
        match = re.search(r"Error\(Contract,\s*#(\d+)\)", primary_error)

        if match:
            try:
                result.error = OracleShieldError(int(match.group(1)))
            except ValueError:
                pass  # Unknown enum value: retain the exception string


@dataclass
class NeedsMoreSignaturesResult:
    error: bool = False


@contextmanager
def expect_needs_more_signatures() -> Iterator[NeedsMoreSignaturesResult]:
    """
    context returning true if NeedsMoreSignaturesError was raised,  None if otherwise
    usage:
    ```
    with expect_needs_more_signature() as error_returned:
        # tested code
    ```
    """
    result = NeedsMoreSignaturesResult()
    try:
        yield result
    except NeedsMoreSignaturesError:
        result.error = True
