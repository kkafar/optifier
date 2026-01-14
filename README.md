# Optifier

 Proc macro crate: `Partial` derive
 
> [!note] This crate is early development phase. I use it only in my personal projects
> and update it as needed. 

 Derive `Partial` on a struct `Foo` to generate a new struct named `FooPartial`
 where every field type is wrapped in `Option<T>` unless it is already an `Option<T>`.
 Only structs with named fields are accepted; tuple and unit structs are not supported **yet**.

 Example:
 ```rust
 use optifier;

 #[derive(optifier::Partial)]
 #[optifier::partial_derive(Debug, Clone)]
 pub struct Foo {
     a: i32,
     b: Option<String>,
     pub c: Vec<u8>,
 }
 ```
 expands to:
 ```rust
 #[derive(Debug, Clone)]
 pub struct FooPartial {
     a: Option<i32>,
     b: Option<String>, // stays as-is
     pub c: Option<Vec<u8>>,
 }
 ```

## `#[optifier::partial_derive(...)]` attribute

The `#[optifier::partial_derive(...)]` attribute configures which traits are derived
for the generated `*Partial` type.

- It is applied to the original struct, together with `#[derive(optifier::Partial)]`.
- It accepts a comma-separated list of trait paths (e.g. `Debug`, `Clone`, `PartialEq`,
  or fully qualified paths).
- Whatever traits you list there are emitted as a `#[derive(...)]` on the generated
  `<OriginalName>Partial` type.

Example:

```rust
use optifier;

#[derive(optifier::Partial)]
#[optifier::partial_derive(Debug, Clone, PartialEq)]
pub struct Foo {
    a: i32,
    b: Option<String>,
}
```

generates roughly:

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct FooPartial {
    a: Option<i32>,
    b: Option<String>,
}
```
