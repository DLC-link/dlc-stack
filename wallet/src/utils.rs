use dlc_manager::{
    contract::{numerical_descriptor::NumericalDescriptor, ContractDescriptor},
    payout_curve::{
        PayoutFunction, PayoutFunctionPiece, PayoutPoint, PolynomialPayoutCurvePiece,
        RoundingInterval, RoundingIntervals,
    },
};
use dlc_messages::oracle_msgs::{DigitDecompositionEventDescriptor, EventDescriptor};
use dlc_trie::OracleNumericInfo;

pub(crate) const NB_DIGITS: usize = 14;
pub(crate) const BASE: usize = 2;

pub(crate) fn get_numerical_contract_info(
    accept_collateral: u64,
    offer_collateral: u64,
    total_outcomes: u64,
    nb_attestors: usize,
) -> Result<(EventDescriptor, ContractDescriptor), dlc_manager::error::Error> {
    let event_descriptor =
        EventDescriptor::DigitDecompositionEvent(DigitDecompositionEventDescriptor {
            base: BASE as u16,
            is_signed: false,
            unit: "btc/usd".to_string(),
            precision: 1,
            nb_digits: NB_DIGITS as u16,
        });

    let descriptor = get_numerical_contract_descriptor(
        accept_collateral,
        offer_collateral,
        total_outcomes,
        nb_attestors,
    )?;

    Ok((event_descriptor, descriptor))
}

pub(crate) fn get_numerical_contract_descriptor(
    accept_collateral: u64,
    offer_collateral: u64,
    total_outcomes: u64,
    nb_attestors: usize,
) -> Result<ContractDescriptor, dlc_manager::error::Error> {
    Ok(ContractDescriptor::Numerical(NumericalDescriptor {
        payout_function: PayoutFunction::new(get_polynomial_payout_curve_pieces(
            accept_collateral,
            offer_collateral,
            total_outcomes,
        )?)?,
        rounding_intervals: RoundingIntervals {
            intervals: vec![RoundingInterval {
                begin_interval: 0,
                rounding_mod: 1,
            }],
        },
        oracle_numeric_infos: OracleNumericInfo {
            base: BASE,
            nb_digits: vec![NB_DIGITS; nb_attestors],
        },
        difference_params: None,
    }))
}

pub(crate) fn get_polynomial_payout_curve_pieces(
    accept_collateral: u64,
    offer_collateral: u64,
    total_outcomes: u64,
) -> Result<Vec<PayoutFunctionPiece>, dlc_manager::error::Error> {
    let total_collateral: u64 = accept_collateral + offer_collateral;
    Ok(vec![
        PayoutFunctionPiece::PolynomialPayoutCurvePiece(PolynomialPayoutCurvePiece::new(vec![
            PayoutPoint {
                event_outcome: 0,
                outcome_payout: 0,
                extra_precision: 0,
            },
            PayoutPoint {
                event_outcome: total_outcomes,
                outcome_payout: total_collateral,
                extra_precision: 0,
            },
        ])?),
        PayoutFunctionPiece::PolynomialPayoutCurvePiece(PolynomialPayoutCurvePiece::new(vec![
            PayoutPoint {
                event_outcome: total_outcomes,
                outcome_payout: total_collateral,
                extra_precision: 0,
            },
            PayoutPoint {
                event_outcome: max_value() as u64,
                outcome_payout: total_collateral,
                extra_precision: 0,
            },
        ])?),
    ])
}

pub(crate) fn max_value() -> u32 {
    2_u32.pow(NB_DIGITS as u32) - 1
}
