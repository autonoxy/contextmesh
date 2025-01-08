use std::collections::{HashMap, HashSet};

pub fn language_kinds_map() -> HashMap<&'static str, HashSet<&'static str>> {
    let mut map = HashMap::new();

    map.insert(
        "rust",
        HashSet::from([
            "function_item",      // fn foo() { ... }
            "method_declaration", // impl Foo { fn bar() { ... } }
            "trait_item",         // trait Foo { ... }
            "impl_item",          // impl Foo { ... } or impl SomeTrait for Foo { ... }
            "struct_item",        // struct Foo { ... }
            "enum_item",          // enum Foo { ... }
            "field_declaration",  // For struct fields
            "static_item",        // static FOO: ...
            "const_item",         // const BAR: ...
        ]),
    );

    map
}
