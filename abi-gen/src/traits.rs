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

use crate::*;
use liquid_macro::seq;
use liquid_prelude::{string::String, vec::Vec};
#[cfg(feature = "contract")]
use liquid_primitives::__Liquid_Getter_Index_Placeholder;

pub trait TypeToString {
    fn type_to_string() -> String;
}

pub trait GenerateParamAbi {
    fn generate_ty_name() -> String;

    fn generate_internal_ty_name() -> String;

    fn generate_param_abi(name: String) -> ParamAbi;
}

pub trait FnOutputBuilder {
    fn output(&mut self, param_abi: ParamAbi);
}

pub trait GenerateOutputs {
    fn generate_outputs<B>(builder: &mut B)
    where
        B: FnOutputBuilder;
}

impl<T> GenerateParamAbi for T
where
    T: TypeToString,
{
    fn generate_ty_name() -> String {
        T::type_to_string()
    }

    fn generate_internal_ty_name() -> String {
        T::type_to_string()
    }

    fn generate_param_abi(name: String) -> ParamAbi {
        TrivialAbi::new(
            Self::generate_ty_name(),
            Self::generate_internal_ty_name(),
            name,
        )
        .into()
    }
}

impl<T> GenerateOutputs for T
where
    T: TypeToString,
{
    fn generate_outputs<B>(builder: &mut B)
    where
        B: FnOutputBuilder,
    {
        let param_abi = <Self as GenerateParamAbi>::generate_param_abi("".into());
        builder.output(param_abi);
    }
}

impl<T> GenerateParamAbi for Vec<T>
where
    T: GenerateParamAbi,
{
    fn generate_ty_name() -> String {
        let mut sub_ty = <T as GenerateParamAbi>::generate_ty_name();
        sub_ty.push_str("[]");
        sub_ty
    }

    fn generate_internal_ty_name() -> String {
        let mut internal_sub_ty = <T as GenerateParamAbi>::generate_internal_ty_name();
        internal_sub_ty.push_str("[]");
        internal_sub_ty
    }

    fn generate_param_abi(name: String) -> ParamAbi {
        let param_abi = <T as GenerateParamAbi>::generate_param_abi(name.clone());
        let components = match param_abi {
            ParamAbi::Composite(composite_abi) => composite_abi.components,
            _ => Vec::new(),
        };

        CompositeAbi {
            trivial: TrivialAbi::new(
                Self::generate_ty_name(),
                Self::generate_internal_ty_name(),
                name,
            ),
            components,
        }
        .into()
    }
}

impl<T> GenerateOutputs for Vec<T>
where
    T: GenerateParamAbi,
{
    fn generate_outputs<B>(builder: &mut B)
    where
        B: FnOutputBuilder,
    {
        let param_abi = <Self as GenerateParamAbi>::generate_param_abi("".into());
        builder.output(param_abi);
    }
}

impl<T, const N: usize> GenerateParamAbi for [T; N]
where
    T: GenerateParamAbi,
{
    fn generate_ty_name() -> String {
        let mut sub_ty = <T as GenerateParamAbi>::generate_ty_name();
        sub_ty.push_str(&format!("[{}]", N));
        sub_ty
    }

    fn generate_internal_ty_name() -> String {
        let mut internal_sub_ty = <T as GenerateParamAbi>::generate_internal_ty_name();
        internal_sub_ty.push_str(&format!("[{}]", N));
        internal_sub_ty
    }

    fn generate_param_abi(name: String) -> ParamAbi {
        let param_abi = <T as GenerateParamAbi>::generate_param_abi(name.clone());
        let components = match param_abi {
            ParamAbi::Composite(composite_abi) => composite_abi.components,
            _ => Vec::new(),
        };

        CompositeAbi {
            trivial: TrivialAbi::new(
                Self::generate_ty_name(),
                Self::generate_internal_ty_name(),
                name,
            ),
            components,
        }
        .into()
    }
}

impl<T, const N: usize> GenerateOutputs for [T; N]
where
    T: GenerateParamAbi,
{
    fn generate_outputs<B>(builder: &mut B)
    where
        B: FnOutputBuilder,
    {
        let param_abi = <Self as GenerateParamAbi>::generate_param_abi("".into());
        builder.output(param_abi);
    }
}

#[cfg(feature = "contract")]
impl GenerateParamAbi for __Liquid_Getter_Index_Placeholder {
    fn generate_ty_name() -> String {
        String::new()
    }

    fn generate_internal_ty_name() -> String {
        String::new()
    }

    fn generate_param_abi(_: String) -> ParamAbi {
        ParamAbi::None
    }
}

macro_rules! impl_generate_outputs_for_tuple {
    ($first:tt,) => {
        impl<$first> GenerateOutputs for ($first,)
        where
            $first: GenerateParamAbi
        {
            fn generate_outputs<B>(builder: &mut B)
            where
                B: FnOutputBuilder,
            {
                builder.output(
                    {
                        let param_abi = <$first as GenerateParamAbi>::generate_param_abi("".into());
                        param_abi.into()
                    }
                );
            }
        }
    };
    ($first:tt, $($rest:tt,)+) => {
        impl<$first, $($rest,)+> GenerateOutputs for ($first, $($rest,)+)
        where
            $first: GenerateParamAbi,
            $(
                $rest: GenerateParamAbi,
            )*
        {
            fn generate_outputs<B>(builder: &mut B)
            where
                B: FnOutputBuilder
            {
                builder.output(
                    {
                        let param_abi = <$first as GenerateParamAbi>::generate_param_abi("".into());
                        param_abi.into()
                    }
                );
                $(
                    builder.output(
                        {
                            let param_abi = <$rest as GenerateParamAbi>::generate_param_abi("".into());
                            param_abi.into()
                        }
                    );
                )+
            }
        }

        impl_generate_outputs_for_tuple!($($rest,)+);
    }
}

seq!(N in 0..16 {
    impl_generate_outputs_for_tuple!(#(T#N,)*);
});

impl<T> GenerateParamAbi for Option<T>
where
    T: GenerateParamAbi,
{
    fn generate_ty_name() -> String {
        String::from("enum")
    }

    fn generate_internal_ty_name() -> String {
        String::from("enum.Option")
    }

    fn generate_param_abi(name: String) -> ParamAbi {
        let some = ParamAbi::Composite(CompositeAbi {
            trivial: TrivialAbi::new(String::from("Some"), String::new(), String::new()),
            components: vec![<T as GenerateParamAbi>::generate_param_abi(String::new())],
        });
        let none = ParamAbi::Composite(CompositeAbi {
            trivial: TrivialAbi::new(String::from("None"), String::new(), String::new()),
            components: vec![],
        });

        let components = vec![some, none];

        ParamAbi::Composite(CompositeAbi {
            trivial: TrivialAbi::new(
                Self::generate_ty_name(),
                Self::generate_internal_ty_name(),
                name,
            ),
            components,
        })
    }
}

impl<T, E> GenerateParamAbi for Result<T, E>
where
    T: GenerateParamAbi,
    E: GenerateParamAbi,
{
    fn generate_ty_name() -> String {
        String::from("enum")
    }

    fn generate_internal_ty_name() -> String {
        String::from("enum.Result")
    }

    fn generate_param_abi(name: String) -> ParamAbi {
        let ok = ParamAbi::Composite(CompositeAbi {
            trivial: TrivialAbi::new(String::from("Ok"), String::new(), String::new()),
            components: vec![<T as GenerateParamAbi>::generate_param_abi(String::new())],
        });
        let err = ParamAbi::Composite(CompositeAbi {
            trivial: TrivialAbi::new(String::from("Err"), String::new(), String::new()),
            components: vec![<E as GenerateParamAbi>::generate_param_abi(String::new())],
        });

        let components = vec![ok, err];

        ParamAbi::Composite(CompositeAbi {
            trivial: TrivialAbi::new(
                Self::generate_ty_name(),
                Self::generate_internal_ty_name(),
                name,
            ),
            components,
        })
    }
}

macro_rules! count_elements {
    () => (0usize);
    ($x:tt $($xs:tt)*) => (1usize + count_elements!($($xs)*));
}

macro_rules! impl_generate_param_abi_for_tuple {
    ($first:tt,) => {
        impl<$first> GenerateParamAbi for ($first,)
        where
            $first: GenerateParamAbi
        {
            fn generate_ty_name() -> String {
                String::from("tuple")
            }

            fn generate_internal_ty_name() -> String {
                String::from("tuple.Tuple1")
            }

            fn generate_param_abi(name: String) -> ParamAbi {
                let param_abis = vec![<$first as GenerateParamAbi>::generate_param_abi("".to_owned())];
                CompositeAbi {
                    trivial: TrivialAbi::new(
                        Self::generate_ty_name(),
                        Self::generate_internal_ty_name(), name),
                        components: param_abis,
                }
                .into()
            }
        }
    };
    ($first:tt, $($rest:tt,)+) => {
        impl<$first, $($rest),+> GenerateParamAbi for ($first, $($rest),+)
        where
            $first: GenerateParamAbi,
            $($rest: GenerateParamAbi),*
        {
            fn generate_ty_name() -> String {
                String::from("tuple")
            }

            fn generate_internal_ty_name() -> String {
                const COUNT: usize = count_elements!($first $($rest )+);
                let mut internal_ty_name = String::from("tuple.Tuple");
                internal_ty_name.push_str(&COUNT.to_string());
                internal_ty_name
            }

            fn generate_param_abi(name: String) -> ParamAbi {
                let mut param_abis = vec![<$first as GenerateParamAbi>::generate_param_abi("".to_owned())];
                $(
                    param_abis.push(<$rest as GenerateParamAbi>::generate_param_abi("".to_owned()));
                )+
                CompositeAbi {
                    trivial: TrivialAbi::new(
                        Self::generate_ty_name(),
                        Self::generate_internal_ty_name(),
                        name),
                    components: param_abis,
                }
                .into()
            }
        }

        impl_generate_param_abi_for_tuple!($($rest,)+);
    }
}

seq!(N in 0..18 {
    impl_generate_param_abi_for_tuple!(#(T#N,)*);
});
