use crate::hooks::DebugHook;
use refractium::{hook_protocol, protocols::http::Http};

hook_protocol!(
    wrapper: HookedHttp,
    proto: Http,
    hooks: [DebugHook]
);
