use optifier;

#[derive(optifier::Partial)]
struct TestType {
    field_i32: i32,
    field_string: String,
}

fn main() -> Result<(), ()> {
    let _complete = TestType { field_i32: 42, field_string: String::from("Hello world") };
    let partial = TestTypePartial { field_i32_opt: Some(42), field_string_opt: Some(String::from("hello world")) };
    dbg!(partial);
    return Ok(());
}

