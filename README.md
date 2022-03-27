# Fastmem

An allocation library for embedded Rust back by static memory.

## Design

By example:

```rust
#[test]
fn test_alloc() {
    pub static HEAP: Heap = Heap::new();
    pub static HEAP_DATA: RacyCell<[u8; 128]> = RacyCell::new([0; 128]);
    HEAP.init(&HEAP_DATA);
    pub static ALLOC: AllocTmp = AllocTmp::new(&HEAP);
    let alloc = ALLOC.init();

    let n_u8 = alloc.box_new(8u8);
    println!("n_u8 {}", *n_u8);

    let n_u32 = alloc.box_new(32u32);
    println!("n_u32 {}", *n_u32);

    drop(n_u8); // force drop
    drop(n_u32); // force drop

    let n_u8 = alloc.box_new(8u8);
    println!("n_u8 {}", *n_u8);

    let n_u32 = alloc.box_new(32u32);
    println!("n_u32 {}", *n_u32);
}
```

The `Heap` data structure holds two raw pointers to the beginning and end of the heap. These could possibly be tied to link script symbols, but for now we set them based on a static allocation of data, in `HEAP_DATA`.

The actual allocator state is stored in `ALLOC` (referring to the backing store). `ALLOC.init()` returns an API to the initiated allocator, allowing to allocate boxed data.

On `drop` the associated allocation is recycled, into a set of pre-allocated stacks. For the example the set is 128, but this could be generalized (by a const generic).

What makes `fastmem` fast is that data is never really de-allocated but rather re-cycled.

Taken from the implementation:

``` rust
    ...
    pub fn box_new<T>(&'static self, t: T) -> Box<T> {
        let index = size_of::<T>();
        println!("box_new, index {}", index);

        match self.free_stacks[index].pop() {
            Some(node) => {
                println!("found node, n_addr {:x}", node.data);
                let data = unsafe { &mut *(&mut *(node.data as *mut T)) };
                *data = t;

                Box::new(data, self, node)
            }
            None => {
                println!("new allocation");
                let n = self.heap.alloc(t);
                let n_addr = n as *const T as usize;
                println!("n_addr {:x}", n_addr);
                let n_node = self.heap.alloc(Node::new(n_addr));

                Box::new(n, self, n_node)
            }
        }
    }
    ...
```
`index` is computed based on the size of `T` (the size of the data we want to allocate). We select the appropriate stack by indexing the array `self.free_stacks`, and `pop`.

If `Some(node)` was found, we re-use the corresponding store (`node.data`) and return a new `Box`, else (if no free memory of size `T` is found), we allocate both the store `n` and a `Node` for book-keeping. 

The "hot path" (re-cycling) is kept to a minimal, and thus should be very fast (my goal is 20 cycles, but not yet measured, so we will see). The cold path is also fairly efficient amounting to two allocations (for `data` and `Node` with possible padding). Here I assume something like 40 cycles.

## Further improvements

One could pre-allocate a set of `Nodes` into a separate data structure which would make "cold path" faster. However, this would require enforcing a hard limit to the number of allocations. 

For now indexing is solely based on the size, e.g. a `T` of size 13 would look at `free_stacks[13]`. From a CPU performance perspective this is likely the fastest we can do. The downside is that we waste some memory for `Stack`s with unlikely sizes, (13 is not the most common size, right). So we could round up the size to next power of 2, and then make a log2 of the rounded size. Each `Stack` is represented by a `head` node () 

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct Stack {
    pub head: Cell<Option<NonNull<Node>>>,
}
```

`Cell<Option<NonNull<Node>>>` is fairly storage efficient. `Cell` is transparent, and so is `Option` for a `NonNull` pointer. The `Node` is also small:

```rust
pub struct Node {
    pub data: usize,
    next: Option<NonNull<Node>>,
}
```
Each `Node` holds a pointer to the `data` (32 bits) and a `next` field (another 32 bits). (Observation: We could use a `NonNull` pointer for the `data`, might be cleaner). In any case on a 32-bit system the cost is 8 bytes. So the memory overhead is only 8 bytes for each allocation + 4 times the number of "slots" (determined by the max size of allocatable elements.)

As mentioned the allocator could be const generic to the max-size (`free_stacks: [const N; ...])`. We could check with a `const_assert` that allocations are in range and use unchecked lookup. Rust will (eventually) prove the `const_assert` at compile time, and allow some run-time code for checked array lookup to be optimized out.

## Limitations and intended "semi static" use

As seen there is no true de-allocation, just re-cycling for sake of performance. In highly dynamic workloads this will be wasteful of resources (memory) but the intention is to use `fastmem` for hard real-time systems, which are typically less dynamic regarding allocation requirements. A typical use case is to store messages (closures) or async futures. While these are dynamic in size (unsized at compile time), there will be limited set of recurring fix sized messages and futures in the system. For a fixed set of futures (async tasks, these can be dynamically allocated at startup).

## Disclaimer: A two day hack

The allocator was wrapped up as a prototype for evaluation, not yet properly tested (and not even `#[no_std]` due to tracing output currently).













