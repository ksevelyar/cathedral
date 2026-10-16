* don't use #[allow] to bypass clippy
* rust edition is 2024, no mod.rs allowed
* only feature tests allowed
* respect aplication boundaries and public api
* no defensive programming allowed, non recoverable errors should panic if invariant violated
