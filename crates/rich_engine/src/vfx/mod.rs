mod bindings;

pub fn test_vfx()
{
    use crate::vfx::bindings::TestCall;
    unsafe {
        let ret = TestCall(1);
        assert_eq!(ret, 2);
    }
}

#[cfg(test)]
mod tests {
    use crate::vfx::bindings::TestCall;

    #[test]
    fn it_works() {
        unsafe {
            let ret = TestCall(1);
            assert_eq!(ret, 3);
        }
    }
}