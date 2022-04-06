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
    let b :Box<u8> = fm.new(1u8);
    hprintln!("{:?}", b);
    
    drop(b); // explicit drop the box

    // re-allocation of a dropped box
    let b :Box<u8> = fm.new(1u8);
    hprintln!("{:?}", b);

    (Shared {}, Local {}, init::Monotonics())
}
```

The corresponding output:
``` shell
Box { node: Node { next: Some(0x2000040c), data: 1 } }
Box { node: Node { next: Some(0x2000040c), data: 1 } }
```

The SRAM is located at `0x2000_0000`, we have allocated 1kB for the data storage. The address `0x2000040c` is pointing to the internal re-allocation stack for the allocated box (more on that later).

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

Initialization returns a reference to an initialized overlay `FastMem` essentially a union of `FastMemStore`, but Rust (currently) rejects unions over const generics even if invariant/preserved.

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

The `heap` actually holds uninitialized data but the public API (`new` box implementation) ensures that allocated data is initialized before dereferenced. So for convenience of the implementation, the `MaybeUninit` is assumed initiated.

The `FastMem` data structure holds `start` and `end` of the `heap` (data storage). (Their initial values could could possibly be tied to link script symbols to automatically allocate the memory region.) 

A raw allocation is implemented as:

``` rust
fn alloc<T>(&'static self) -> Option<&mut T> {
    let mut start = self.start.get();

    let size = size_of::<T>();
    let align = align_of::<T>();
    let spill = start % align;

    if spill != 0 {
        start += align - spill;
    }

    let new_start = start + size;
    if new_start > self.end.get() {
        None
    } else {
        let r: &mut T = unsafe { transmute::<_, &mut T>(start) };
        self.start.set(new_start);
        Some(r)
    }
}
```

We ensure that the allocated data is aligned by padding (`spill`). We transmuted the allocated space to a mutable reference `&mut T`. In general this is unsound but the reference is always initialized before dereferenced as shown below.

The user facing API provides both a `try_new(&'static,t:T) -> Option<Box<t>>` and the `new(&'static,t:T) -> Box<T>` which panics on an out of memory condition.  

The implementation:
``` rust
pub fn try_new<T>(&'static self, t: T) -> Option<Box<T>> {
    let index = size_of::<T>();
    let stack = &self.stacks[index];

    let node: &mut Node<T> = match stack.pop() {
        Some(node) => {
            // check alignment of data
            if &node.data as *const _ as usize % align_of::<T>() != 0 {
                panic!("illegal alignment @{:p}", node);
            };

            node
        }
        None => {
            self.alloc()?
        }
    };
    node.data = t; 
    node.next = unsafe { transmute(stack) };

    Some(Box::new(node))
}
```

The `index` is determined from the size of `T` and used to `pop` the corresponding recycling stack (`self.stack[index]`). If a `node` is found we check its alignment (as an extra step of precaution). If the recycling stack is empty we create a new allocation.

The `node.data = t` is initialized and the `next` field is set to point to the head of the recycling stack. The type is erased by the transmute (the type of the data is irrelevant for the recycling, only the size is of interest).

Later when the returned box is dropped, the `node` is `push`ed to the recycling stack (pointed to by the `next` field). Also here we erase the type using transmute.

``` rust
impl<T> Drop for Box<T> {
    fn drop(&mut self) {
        let stack: &Stack = unsafe { transmute(self.node.next) };
        stack.push(self.node);
    }
}
```
---

## CPU Performance

What makes `fastmem` fast is that data is never really de-allocated but rather re-cycled. All operations are constant time besides:

- FastMemStore `init`, which is linear to `S` (the maximum allocation size). This is done once.
- FastMem `new`/`try_new`, where the `node.data = t` (value assigned by move) cost is determined by `T`.

On first allocation, data is allocated from the `heap` (approximately 25 cycles), an re-allocation (the common case) we merely `pop` an existing stack (< 20 cycles).

---

## Memory Performance

The static store contains the data (`heap:[u8;N]`), `start/end` (both `usize`), and an array of `stacks : [Stack;S]` (where each `Stack` is `usize`). Each allocated `node` has a an overhead of `usize`. (`usize` has size and alignment of 4 for the Cortex-M 32 bit architecture.) 

Rust ensures that alignments are even powers of 2 (1, 2, 4, ... etc). The implementation by `stack: [Stack;S]`, trades memory for performance (e.g. `index == 5` will never be used, but on the other hand lookup is constant time.) Assuming `S == 128` we have only 7 valid/distinct sizes (1, 2, 4, 8, 16, 32, 64), while we are using 128*4 = 512 bytes of memory. 

--

## Further performance improvements

The CPU performance is strikingly good, perhaps even *zero-cost* in Rust terms. However we are not quite there yet. E.g., the `index` lookup is range checked. Hopefully Rust type system will in the future allow the size of `T` to be checked against `S` at compile time (`where size_of::<T> < S` or similar). Alternatively, we could adopt unchecked array indexing (and apply bounds checking at compile time, e.g. by symbolic execution).

We could also mitigate the memory overhead by relying on the Rust alignment guarantees (even powers of 2), and just count trailing zeros. This can be done in assembly by the `CNZ` instruction. In this way `stacks: [Stacks; S]` would imply size of the maximum allocatable block would be 2 to the power of `S`, and we would waste memory only in cases some block size is never used by the application. However, as as a first proof of concept we accept the memory overhead.

### Considerations

The efficiency (both memory and CPU) relies on the recycling of fixed sized blocks. Blocks will never be collected/merged. This may give rise to fragmentation issues (we can run out of memory, while at the same time having free blocks available in other recycling stacks).

While there are numerous possible ways to implement de-fragmentation, the goal with `fastmem` is to provide a simple, light weight and fast allocator with constant time guarantees suitable to statically analyzable hard real-time systems. 

In practice such systems are typically well behaved (with repetitive allocation patterns). E.g., buffers for DMA operations are circulated between the driver (allocating the buffer), to the user task(s) consuming the data, and/or the other way around. The number of outstanding buffers is typically small with a bounded maximum in accordance to governing real-time constraints. Other examples include communication where we address recurring packages with a limited set of unique package sizes. 

An RTIC application executes in two phases, the `init` task used to do static configuration and the `run` phase where tasks are dynamically executed. In case the allocation patterns during `init` and `run` differs, we might have `stacks` of free memory that will never be used after `init`. This problem might be solved by  applying de-fragmentation between the `init` and `run` phase. Even box relocation should be fine as long as data has is not pinned (`Pin<T>`), but that has to be investigated further.

---

## Soundness

Whereas the implementation is straightforward and the use of unsafe code is limited to a small set of transmute operations, ensuring soundness is still non-trivial. The current implementation is to be considered experimental, and further work is needed regarding:

- Drop semantics (are we correctly handling all possible cases, e.g., leaking memory, zero-sized types etc.).
- Alignment (for now we take a defensive approach applying run-time verification of actual alignment).
- Sync/Send type bounds (are we following Rust semantics for all cases).
- Likely more...

---

## License and Contributions

License has not yet been determined. Most likely it will follow the dual MIT/Apache licensing scheme.














