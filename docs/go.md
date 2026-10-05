# Go documentation

These are historical Go learning notes. The active search service now uses [Rust and C++](../apps/search/rust/README.md); Go is no longer used by Compose.

```
:= 
```
- means that it infers its type

## Slice
In Go, a slice is a view into an underlying array. It contains:
- a pointer to the array's data
- Length: how many elements the slice contains
- Capacity: how many elements it can hold from its starting position before needing a larger array
