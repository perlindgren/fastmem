use core::fmt;

#[derive(Copy, Clone)]
pub struct Node {
    data: usize,
    next: Option<&'static Node>,
}

pub struct List {
    head: usize,
}

impl List {
    // this works for just push one node to the top
    pub fn push(&mut self, &) -> List {
        List::Node(self, m)
    }

    pub fn pop(&mut self) -> &'static List {
        match self {
            List::Node(next, data) => (*List::to_list(*next), data),
            _ => panic!(),
        }
    }

    pub fn to_raw(&self) -> *const List {
        &*self
    }

    pub fn to_list(l: *const List<'a>) -> &List<'a> {
        unsafe { &*(l as *const List<'a>) }
    }
}

impl<'a> fmt::Debug for List<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                List::Node(l, m) => format!("Node ({:?}, {:?})", m, List::to_list(*l)),
                _ => "Null".to_string(),
            }
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use core::mem::size_of_val;

    #[test]
    fn test_list() {
        let l = List::Null;
        println!("l {:?}", l);

        let l = l.push(&[1]);
        println!("l {:?}", l);

        let l = l.push(&[2]);
        println!("l {:?}", l);

        let (l, d) = l.pop();
        println!("l {:?}, d {:?}", l, d);

        let (l, d) = l.pop();
        println!("l {:?}, d {:?}", l, d);
    }

    // #[test]
    // fn test_m() {
    //     let mut mem: Mem<16, 2, 2> = Mem::new();
    //     mem.init();

    //     println!("m {:?}", mem);
    // }

    #[test]
    fn test() {
        let i = 1;
        let t = move |j: i32| j + i;

        println!("t {}", size_of_val(&t));
    }
}
