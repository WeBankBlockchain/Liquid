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

use cfg_if::cfg_if;
use derive_more::From;
use serde::Serialize;

pub mod traits;
mod type_to_string;

#[derive(Serialize)]
pub struct TrivialAbi {
    #[serde(rename = "type")]
    pub ty: String,
    #[serde(skip_serializing_if = "::std::string::String::is_empty")]
    #[serde(rename = "internalType")]
    pub internal_ty: String,
    #[serde(skip_serializing_if = "::std::string::String::is_empty")]
    pub name: String,
}

impl TrivialAbi {
    pub fn new(ty: String, internal_ty: String, name: String) -> Self {
        TrivialAbi {
            ty,
            internal_ty,
            name,
        }
    }
}

#[derive(Serialize)]
pub struct CompositeAbi {
    #[serde(flatten)]
    pub trivial: TrivialAbi,
    #[serde(skip_serializing_if = "::std::vec::Vec::is_empty")]
    pub components: Vec<ParamAbi>,
}

#[derive(Serialize, From)]
#[serde(untagged)]
pub enum ParamAbi {
    Composite(CompositeAbi),
    Trivial(TrivialAbi),
    None,
}

cfg_if! {
    if #[cfg(feature = "contract")] {
        mod contract;
        pub use contract::*;
    } else if #[cfg(feature = "collaboration")] {
        mod collaboration;
        pub use collaboration::*;
    }
}
