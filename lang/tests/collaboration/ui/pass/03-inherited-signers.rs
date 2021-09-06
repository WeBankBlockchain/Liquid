use liquid_lang as liquid;

#[liquid::collaboration]
mod noop {
    #[liquid(contract)]
    pub struct Foo {
        #[liquid(signers)]
        addr: Address,
    }

    #[liquid(contract)]
    pub struct Bar {
        #[liquid(signers = inherited)]
        foo: Foo,
    }
}

fn main() {}
