use anyhow::Result;
use rstest::fixture;

use crate::header_behaviour::state::build_state;
use crate::HeaderState;

#[fixture]
pub(crate) fn validated_state() -> HeaderState {
    build_state().expect("failed to initialise header state")
}

#[fixture]
pub(crate) fn validated_state_result() -> Result<HeaderState> {
    build_state()
}
