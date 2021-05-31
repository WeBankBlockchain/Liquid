use liquid_lang as liquid;

#[liquid::contract]
mod noop {
    #[liquid(storage)]
    struct Noop {}

    #[liquid(methods)]
    impl Noop {
        pub fn new(&mut self, value: f32) {}

        pub fn noop(&self) {}
    }
}

fn main() {}
