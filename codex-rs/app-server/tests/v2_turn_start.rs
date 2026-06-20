// Independent integration test binary for the v2 turn_start group.
// turn_start analytics tests import helpers from this sibling module.
mod analytics {
    include!("suite/v2/analytics.rs");
}

mod turn_start {
    include!("suite/v2/turn_start.rs");
}
