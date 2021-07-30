// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![allow(dead_code)]
#![allow(unknown_lints)]

pub(crate) mod api;
pub(crate) mod backend;
pub(crate) mod calldata;
pub mod engine;
pub mod error;

pub use self::{
    api::{
        call, emit, finish, get_address, get_asset_balance, get_call_data, get_caller,
        get_external_code_size, get_not_fungible_asset_ids, get_not_fungible_asset_info,
        issue_fungible_asset, issue_not_fungible_asset, now, register_asset, revert,
        transfer_asset,
    },
    backend::CallMode,
};

#[cfg(any(feature = "std", test))]
pub use self::engine::off_chain::test_api as test;
