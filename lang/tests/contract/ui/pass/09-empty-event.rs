use liquid_lang as liquid;

#[liquid::contract]
mod noop {
    #[liquid(storage)]
    struct Noop {}

    #[liquid(event)]
    struct TestEvent {}

    #[liquid(methods)]
    impl Noop {
        pub fn new(&mut self) {}

        pub fn noop(&self) -> () {
            self.env().emit(TestEvent {});
        }
    }
}

fn main() {}
