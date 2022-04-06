# Fastmem

An experimental allocator for embedded Rust.

Design goals, the allocator should: 

- provide fast and predictable allocations for sized types, 
- work on stable rust, and 
- be reasonably memory efficient.

Goal fulfillment:

- O(1) for uninitialized allocations. 
- Initialization is Rust zero-cost, no extra OH added.
- Works for all sized data types (including closures).
- Reasonably memory efficient.

---

## Benchmark

Benchmarks are compiled for ARM Cortex M4 (stm32f411), running at stock settings (16 MHz no flash memory wait states). The measurements include measuring overhead capturing the DWT cycle counter, (measuring overhead add approximately 5 clock cycles to the actual execution time). The measurements also include initialization of the correspond data type. The current implementation also verifies actual alignment at run-time. The allocator is simple and light weight by design. As seen below the difference between Debug/dev builds and release builds are marginal.

| Debug/dev         | u8  | u32 | u64 |
| -----------       | --- | --- | --- |
| First allocation  | 33  | 31  | 37  |
| Re-allocation     | 20  | 21  | 24  |
| Drop              | 9   | 9   | 9   |
      
| Release           | u8  | u32 | u64 |
| -----------       | --- | --- | --- |
| First allocation  | 31  | 31  | 34  |
| Re-allocation     | 21  | 21  | 24  |
| Drop              | 9   | 9   | 9   |
   

---

## Usage

The allocator relies on a static backing storage. 

``` rust
  FastMemStore<const N: usize, S: usize>
```
Where `N` is the size (in bytes), and `S` is the maximum size of an allocation. Using RTIC we can create an fastmem store as follows:

```rust
 #[init(local = [fm_store: FastMemStore<1024, 128> = FastMemStore::new()])]
```

Here we have reserved 1024 bytes with a maximum allocation of 128 bytes. 

A minimal usage example:

```rust
fn init(mut cx: init::Context) -> (Shared, Local, init::Monotonics) {

    // create an initialized fastmem allocator from the store
    let fm: &'static FastMem<1024, 128> = cx.local.fm_store.init();

    // create a boxed value of type u8 (first allocation)
    let b :Box<u8> = fm.box_new(1u8);
    hprintln!("{:?}", b);
    
    drop(b); // explicit drop the box

    // re-allocation of a dropped box
    let b :Box<u8> = fm.box_new(1u8);
    hprintln!("{:?}", b);

    (Shared {}, Local {}, init::Monotonics())
}
```

The corresponding output:
``` shell
Box { node: Node { next: Some(0x2000040c), data: 1 } }
Box { node: Node { next: Some(0x2000040c), data: 1 } }
```

The SRAM is located at `0x2000_0000`, we have allocated `1k` for the allocator data. The address `0x2000040c` is pointing to the re-allocation stack for the allocated box (more on that later).

As seen the second allocation is re-using the dropped box.

In a sequential context Rust+LLVM is able to see through local re-uses and further optimize the fastmem implementation (thus the benchmarking above is done in a concurrent setting, using separated tasks).

The `Box<T>` de-references to a `&'static mut T`, thus we can store boxed allocations in RTIC resources (and messages) in a similar fashion as [heapless::pool](https://docs.rs/heapless/latest/heapless/pool/index.html). (Compared to `pool`, allocations may be of different types/sizes.)

---

## Design

Internally the backing storage is implemented as:

```rust
/// Uninitialized representation of backing storage.
///
/// Type parameters:
///
/// const N:usize, the size of the storage in bytes.
/// const S:usize, allocatable sizes [0..S] in bytes.              
#[repr(C)]
pub struct FastMemStore<const N: usize, const S: usize> {
    heap: MaybeUninit<UnsafeCell<[u8; N]>>,
    start: Cell<usize>,
    end: Cell<usize>,
    stacks: MaybeUninit<[Stack; S]>,
}
```

Initialization returns a reference to an initialized overlay `FastMem` essentially a union of `FastMemStore`, but Rust does not allow unions over const generics (even if constants are provably preserved, transmute to our rescue).

```rust
/// Initialized representation of the backing storage.
#[repr(C)]
pub struct FastMem<const N: usize, const S: usize> {
    heap: UnsafeCell<[u8; N]>,
    start: Cell<usize>,
    end: Cell<usize>,
    stacks: [Stack; S],
}
```

(Remark, the `heap` actually holds uninitialized data but the `new_box` implementation ensures that allocated data is initialized before dereferenced.)

---

Todo ...


<!-- 
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

As seen there is no true de-allocation, just re-cycling for sake of performance. For highly dynamic workloads this will be wasteful of resources (memory) but the intention is to use `fastmem` for hard real-time systems, which are typically less dynamic regarding allocation requirements. A typical use case is to store messages (closures) or async futures. While these are dynamic in size (unsized at compile time), there will be limited set of recurring fix sized messages and futures in the system. For a fixed set of futures (async tasks, these can be dynamically allocated at startup).

## Disclaimer: A two day hack

The allocator was wrapped up as a prototype for evaluation, not yet properly tested (and not even `#[no_std]` due to tracing output currently). -->













