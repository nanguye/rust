type FunPtr = fn(i32)->i32;
fn foo(a: i32) -> i32
{
    println!("{}", a);
    return a;
}
#[derive(Debug)]
struct TargetObject
{
    _fun: FunPtr,
}
impl Drop for TargetObject {
    fn drop(&mut self) {
        println!("Destructor running for a TargetObject");
    }
}
fn main()
{
    unsafe
    {
        let t_object = TargetObject{ _fun: foo};
        let pointer: *const TargetObject = &t_object;
        ((*pointer.offset(1))._fun)(1);
        println!("Ending");
    }
}
