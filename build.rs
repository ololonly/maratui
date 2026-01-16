fn main() {
    // Emits Cargo instructions to propagate ESP-IDF system environment
    // variables into the build so the Rust code and the ESP-IDF bindings are aligned.
    embuild::espidf::sysenv::output();
}
