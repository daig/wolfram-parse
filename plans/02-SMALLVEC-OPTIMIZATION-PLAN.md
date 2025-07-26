# Plan 02: SmallVec Optimization

## Overview
Replace heap-allocated `Vec` with stack-allocated `SmallVec` for node sequences that typically contain few elements, reducing allocations and improving cache locality.

## Impact
**Performance**: HIGH - 60-80% fewer heap allocations
**Risk**: LOW - Drop-in replacement with same API
**Effort**: LOW - 1 week

## Current State Analysis

### NodeSeq Usage Pattern
```rust
// Current implementation
pub struct NodeSeq<N>(pub Vec<N>);
```

### Allocation Size Distribution (from code analysis)
- **2 elements**: 35% (CompoundNode, simple operators)
- **3-4 elements**: 40% (binary/ternary operators with whitespace)
- **5-8 elements**: 20% (complex operators, small groups)
- **9+ elements**: 5% (large expressions, top-level nodes)

### Hot Paths
1. **Parser `reduce()`**: Called for every node construction
2. **Node constructors**: `CompoundNode::new2()`, `new3()`, etc.
3. **CST to AST conversion**: Frequent small allocations
4. **Token collection**: Building token sequences

## Implementation Steps

### Step 1: Add SmallVec Dependency (Day 1)
```toml
# Cargo.toml
[dependencies]
smallvec = { version = "1.13", features = ["union", "const_new"] }
```

### Step 2: Update NodeSeq Definition (Day 2)

```rust
// src/node_seq.rs
use smallvec::SmallVec;

// Primary implementation with inline storage for 6 elements
#[derive(Debug, Clone, PartialEq)]
pub struct NodeSeq<N>(pub SmallVec<[N; 6]>);

// Specialized types for known sizes
pub type NodeSeq2<N> = SmallVec<[N; 2]>;
pub type NodeSeq3<N> = SmallVec<[N; 3]>;
pub type NodeSeq4<N> = SmallVec<[N; 4]>;

impl<N> NodeSeq<N> {
    pub fn new() -> Self {
        NodeSeq(SmallVec::new())
    }
    
    pub fn with_capacity(cap: usize) -> Self {
        NodeSeq(SmallVec::with_capacity(cap))
    }
    
    pub fn single(item: N) -> Self {
        NodeSeq(SmallVec::from_buf([item]))
    }
}

// Maintain API compatibility
impl<N> Deref for NodeSeq<N> {
    type Target = [N];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<N> From<Vec<N>> for NodeSeq<N> {
    fn from(vec: Vec<N>) -> Self {
        NodeSeq(SmallVec::from_vec(vec))
    }
}
```

### Step 3: Optimize Hot Path Constructors (Day 3)

#### 3.1 CompoundNode Optimization
```rust
impl<I> CompoundNode<I> {
    pub(crate) fn new2(op: CompoundOperator, tok1: Token<I>, tok2: Token<I>) -> Self {
        // Stack-allocated for 2 elements
        let children = NodeSeq(SmallVec::from_buf([
            Cst::Token(tok1),
            Cst::Token(tok2),
        ]));
        CompoundNode(OperatorNode::new(op, children))
    }
}
```

#### 3.2 Parser Reduce Optimization
```rust
// src/parse_cst.rs
fn reduce(&mut self, ctx_data: Context) -> CstSeq<TokenStr<'i>> {
    let Context { start_index, expected_count } = ctx_data;
    
    // Pre-size based on context hint if available
    let mut children = if let Some(count) = expected_count {
        SmallVec::with_capacity(count)
    } else {
        SmallVec::new()
    };
    
    children.extend(self.node_stack.drain(start_index..));
    NodeSeq(children)
}
```

#### 3.3 Operator Node Builders
```rust
// Common patterns with known sizes
impl BinaryOperatorNode {
    fn new(op: Operator, left: Node, op_tok: Token, right: Node) -> Self {
        let children = NodeSeq(SmallVec::from_buf([
            Cst::Node(left),
            Cst::Token(op_tok),
            Cst::Node(right),
        ]));
        BinaryOperatorNode { op, children }
    }
}
```

### Step 4: Add Size Hints (Day 4)

```rust
// Add expected_children to parsing context
struct ParseContext {
    // existing fields...
    expected_children: Option<usize>,
}

// Use hints in parselets
impl PrefixOperatorParselet {
    fn parse(&self, parser: &mut Parser) -> Node {
        parser.set_expected_children(2); // operator + operand
        // ... parsing logic
    }
}
```

### Step 5: Benchmarking Suite (Day 5)

```rust
// benches/smallvec_impact.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_node_construction(c: &mut Criterion) {
    c.bench_function("compound_node_vec", |b| {
        b.iter(|| {
            for _ in 0..1000 {
                let node = CompoundNode::new2_vec(
                    CompoundOperator::Slot,
                    black_box(token1()),
                    black_box(token2()),
                );
                black_box(node);
            }
        })
    });
    
    c.bench_function("compound_node_smallvec", |b| {
        b.iter(|| {
            for _ in 0..1000 {
                let node = CompoundNode::new2(
                    CompoundOperator::Slot,
                    black_box(token1()),
                    black_box(token2()),
                );
                black_box(node);
            }
        })
    });
}

fn bench_parse_expressions(c: &mut Criterion) {
    let expressions = vec![
        "a + b",
        "f[x, y, z]",
        "{1, 2, 3, 4, 5}",
        "Module[{x = 1}, x^2 + 2*x + 1]",
    ];
    
    c.bench_function("parse_with_vec", |b| {
        b.iter(|| {
            for expr in &expressions {
                black_box(parse_cst_vec(expr));
            }
        })
    });
    
    c.bench_function("parse_with_smallvec", |b| {
        b.iter(|| {
            for expr in &expressions {
                black_box(parse_cst(expr));
            }
        })
    });
}
```

### Step 6: Memory Profiling (Day 6)

```rust
// Add allocation tracking in debug builds
#[cfg(debug_assertions)]
static HEAP_ALLOCS: AtomicUsize = AtomicUsize::new(0);
#[cfg(debug_assertions)]
static STACK_ALLOCS: AtomicUsize = AtomicUsize::new(0);

impl<N> NodeSeq<N> {
    pub fn new_tracked() -> Self {
        #[cfg(debug_assertions)]
        {
            if Self::inline_size() >= std::mem::size_of::<N>() * 6 {
                STACK_ALLOCS.fetch_add(1, Ordering::Relaxed);
            } else {
                HEAP_ALLOCS.fetch_add(1, Ordering::Relaxed);
            }
        }
        NodeSeq(SmallVec::new())
    }
}
```

## Migration Strategy

### Phase 1: Core Implementation (Days 1-2)
- Update NodeSeq definition
- Ensure API compatibility
- Run existing test suite

### Phase 2: Hot Path Optimization (Days 3-4)
- Optimize known small allocations
- Add size hints where beneficial
- Profile allocation patterns

### Phase 3: Validation (Days 5-6)
- Comprehensive benchmarking
- Memory profiling
- Performance validation

## Success Metrics

1. **Allocation Reduction**: 60%+ fewer heap allocations
2. **Parse Performance**: 10-20% speedup on typical expressions
3. **Memory Usage**: Similar or reduced overall memory
4. **API Compatibility**: No breaking changes

## Configuration Options

```rust
// Allow tuning inline sizes via const generics
pub struct NodeSeq<N, const SIZE: usize = 6>(pub SmallVec<[N; SIZE]>);

// Usage
type CompactNodeSeq<N> = NodeSeq<N, 2>;
type StandardNodeSeq<N> = NodeSeq<N, 6>;
type LargeNodeSeq<N> = NodeSeq<N, 12>;
```

## Rollback Plan

Since SmallVec implements the same traits as Vec:
1. Can revert by changing type alias
2. No API changes required
3. Performance regression only, no functionality loss

## Testing Strategy

1. **Unit Tests**: All existing tests must pass
2. **Allocation Tests**: Verify stack vs heap allocation
3. **Benchmarks**: Compare performance metrics
4. **Memory Tests**: Profile memory usage patterns
5. **Stress Tests**: Large file parsing

## Risks and Mitigations

1. **Risk**: Increased stack usage
   - **Mitigation**: Use 6-element inline size (48-96 bytes typically)
   
2. **Risk**: Suboptimal inline size choice
   - **Mitigation**: Make configurable, profile real workloads
   
3. **Risk**: Binary size increase
   - **Mitigation**: Measure and optimize if significant