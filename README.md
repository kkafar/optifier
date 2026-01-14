# Optifier

 Proc macro crate: `Partial` derive
 
> [!note] This crate is early development phase. I use it only in my personal projects
> and update it as needed. 

 Derive `Partial` on a struct `Foo` to generate a new struct named `FooPartial`
 where every field type is wrapped in `Option<T>` unless it is already an `Option<T>`.
 Only structs with named fields are accepted; tuple and unit structs are not supported **yet**.

 Example:
 ```rust
 #[derive(Partial)]
 pub struct Foo {
     a: i32,
     b: Option<String>,
     pub c: Vec<u8>,
 }
 ```
 expands to:
 ```rust
 pub struct FooPartial {
     a: Option<i32>,
     b: Option<String>, // stays as-is
     pub c: Option<Vec<u8>>,
 }
 ```
