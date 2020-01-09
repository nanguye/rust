// compile-flags: -C control-flow-guard=checks -C debuginfo=2
// error-pattern:
// failure-status: -1073740791

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
fn unsafeFun(offset: isize)
{
    unsafe
    {
        let t_object = TargetObject{ _fun: foo};
        let pointer: *const TargetObject = &t_object;
        ((*pointer.offset(offset))._fun)(1);
        println!("Ending");
    }
}
fn main()
{
    unsafeFun(1);
}
