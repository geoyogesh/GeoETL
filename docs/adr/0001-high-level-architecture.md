# ADR 0001: High-Level Architecture for GeoETL

## Status

Draft

## Context

We are building GeoETL, a modern CLI tool for spatial data conversion and processing that aims to become a next-generation replacement for GDAL. The project needs a scalable, performant architecture that can handle both single-node and distributed processing of vector geospatial data.

### Problem Statement

GDAL (Geospatial Data Abstraction Library) has been the de facto standard for geospatial data processing for decades. While revolutionary in its time, it faces several challenges in the modern data landscape:

- **Performance Limitations**: Single-threaded processing in many operations limits throughput
- **Legacy Architecture**: Built on decades-old patterns that don't leverage modern hardware capabilities
- **Scalability Constraints**: Limited support for distributed processing of large datasets
- **Memory Management**: Manual memory management leading to potential inefficiencies
- **Developer Experience**: Complex C++ API with steep learning curve
- **Dependency Complexity**: Massive dependency tree making builds difficult

We need an architecture that addresses these limitations while maintaining compatibility with the geospatial ecosystem and providing a path to 5x-10x performance improvements.

## Decision Drivers

1. **Performance**: Must leverage modern hardware (multi-core, SIMD, vectorization)
2. **Scalability**: Must scale from laptop to distributed cluster seamlessly
3. **Safety**: Must eliminate memory safety issues and common programming errors
4. **Maintainability**: Must be easy to understand, extend, and contribute to
5. **Interoperability**: Must work with existing geospatial formats and tools
6. **Developer Experience**: Must provide intuitive CLI and API interfaces
7. **Cloud Native**: Must support cloud storage and distributed compute platforms
8. **Ecosystem Integration**: Leverage and contribute to the GeoRust ecosystem

## Considered Options

### Option 1: C++ with Modern Patterns
- Continue with C++ but use modern C++20/23 features
- Leverage libraries like Arrow C++ for columnar processing
- Build distributed layer on top

**Pros:**
- Familiarity with existing GDAL ecosystem
- Direct access to existing geospatial libraries
- Proven performance capabilities

**Cons:**
- Still requires manual memory management
- Steep learning curve for contributors
- Limited compile-time safety guarantees
- Complex build systems and dependencies

### Option 2: Go with Custom Query Engine
- Use Go for memory safety and concurrency
- Build custom columnar query engine
- Leverage goroutines for parallelism

**Pros:**
- Memory safe with garbage collection
- Good concurrency primitives
- Fast compilation
- Growing geospatial ecosystem

**Cons:**
- Garbage collection overhead for data-intensive operations
- Less mature analytical query engine ecosystem
- Limited SIMD support
- Performance ceiling lower than systems languages

### Option 3: Rust with DataFusion, Ballista, and GeoRust (Selected)
- Use Rust as primary language
- Integrate Apache DataFusion for query execution
- Use DataFusion Ballista for distributed processing
- Strongly leverage GeoRust ecosystem for geospatial operations
- Build geospatial extensions on top

**Pros:**
- Memory safety without garbage collection overhead
- Zero-cost abstractions for performance
- DataFusion provides proven vectorized execution
- Ballista enables distributed processing
- Strong type system catches errors at compile time
- Rich GeoRust ecosystem (geo, geozero, proj, etc.)
- Growing adoption (GeoArrow, GeoParquet)

**Cons:**
- Steeper learning curve than Go
- Smaller pool of Rust developers
- Some formats may require custom implementations
- Longer compile times

## Decision

**We will use Option 3: Rust with DataFusion, Ballista, and GeoRust**

This architecture provides the best balance of performance, safety, and scalability while building on proven technologies in the analytical computing space and the mature GeoRust ecosystem.

## High-Level Architecture

### System Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                         GeoETL CLI                              │
│                    (User Interface Layer)                       │
└───────────────────────────┬─────────────────────────────────────┘
                            │
┌───────────────────────────▼─────────────────────────────────────┐
│                      GeoETL Core                                │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐  │
│  │   Format     │  │   Spatial    │  │   Coordinate Ref    │  │
│  │   Registry   │  │  Operations  │  │   System (CRS)      │  │
│  │              │  │  (GeoRust)   │  │   (proj crate)      │  │
│  └──────────────┘  └──────────────┘  └──────────────────────┘  │
└───────────────────────────┬─────────────────────────────────────┘
                            │
┌───────────────────────────▼─────────────────────────────────────┐
│                   Execution Engine Layer                        │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │           DataFusion Query Engine                          │ │
│  │  ┌──────────────┐  ┌──────────────┐  ┌────────────────┐  │ │
│  │  │   Logical    │→ │  Optimized   │→ │   Physical     │  │ │
│  │  │     Plan     │  │     Plan     │  │     Plan       │  │ │
│  │  └──────────────┘  └──────────────┘  └────────────────┘  │ │
│  └────────────────────────────────────────────────────────────┘ │
└───────────────────────────┬─────────────────────────────────────┘
                            │
                ┌───────────┴───────────┐
                │                       │
┌───────────────▼──────────┐  ┌────────▼──────────────────────────┐
│  Single-Node Execution   │  │  Distributed Execution (Ballista) │
│  ┌────────────────────┐  │  │  ┌──────────────────────────────┐ │
│  │ Thread Pool        │  │  │  │   Scheduler                  │ │
│  │ (Tokio/Rayon)      │  │  │  │   - Query Planning           │ │
│  └────────────────────┘  │  │  │   - Work Distribution        │ │
└──────────────────────────┘  │  └──────────────────────────────┘ │
                              │  ┌──────────────────────────────┐ │
                              │  │   Executor Nodes             │ │
                              │  │   - Parallel Processing      │ │
                              │  └──────────────────────────────┘ │
                              └───────────────────────────────────┘
                                          │
┌─────────────────────────────────────────▼───────────────────────┐
│                      Format I/O Layer                           │
│                    (GeoRust + Custom)                           │
│  ┌──────────────────────────────┐  ┌──────────────────────┐    │
│  │   Vector Formats             │  │   Cloud Storage      │    │
│  │   GeoJSON, GeoParquet, etc   │  │   Adapters           │    │
│  │   (geozero, geojson, etc)    │  │   S3, Azure, GCS     │    │
│  │                              │  │   (object_store)     │    │
│  └──────────────────────────────┘  └──────────────────────┘    │
└─────────────────────────────────────────────────────────────────┘
```

### Core Components

#### 1. GeoETL CLI (`geoetl-cli`)

**Responsibility**: User-facing command-line interface

**Key Features**:
- Command parsing using `clap` library
- Subcommands for different operations (convert, analyze, transform, etc.)
- Progress reporting and user feedback
- Configuration management
- Error reporting with actionable messages

**Example Commands**:
```bash
geoetl convert input.shp output.parquet
geoetl transform --operation buffer --distance 100m input.geojson
geoetl analyze --stats input.geojson
geoetl distributed --cluster config.yaml convert large-dataset.geojson
```

#### 2. GeoETL Core (`geoetl-core`)

**Responsibility**: Core library with spatial operations and abstractions

**Modules**:

- **Format Registry**
  - Plugin system for format readers/writers
  - Format detection and validation
  - Schema inference
  - Integration with `geozero` for streaming I/O

- **Spatial Operations**
  - Vector operations built on `geo` crate (buffer, intersection, union, etc.)
  - Spatial joins and aggregations
  - Advanced geometric algorithms
  - Leverage `geo-types` for geometric primitives
  - Use `rstar` for spatial indexing

- **CRS Management**
  - Coordinate reference system transformations via `proj` crate
  - Datum conversions
  - WKT parsing via `wkt` crate

- **Metadata Handling**
  - Schema management
  - Spatial indexes (R-tree via `rstar`)
  - Statistics and histograms

#### 3. GeoETL Formats (`geoetl-formats`)

**Responsibility**: Format-specific readers and writers

**Vector Formats** (Priority Order):

| Format | Implementation Strategy | Library/Approach |
|--------|------------------------|------------------|
| GeoParquet | Native, high performance | `parquet` + GeoParquet spec |
| GeoJSON | Ubiquitous, text-based | `geojson` crate (GeoRust) |
| FlatGeobuf | Streaming, cloud-optimized | `flatgeobuf` crate (GeoRust) |
| GeoPackage | SQLite-based | `rusqlite` + custom geometry |
| Shapefile | Legacy support | Custom implementation or `shapefile` crate |
| GML | XML-based | `geozero` + XML parser |
| KML | XML-based | Custom implementation |

**Implementation Strategy**:
1. **Strongly leverage GeoRust ecosystem** - Use existing crates as foundation
2. **Use `geozero`** - Zero-copy streaming for vector formats
3. **Write custom formatters** - For formats lacking Rust support
4. **Targeted C library bindings** - Only when necessary (e.g., PROJ for CRS)
5. **NO GDAL dependency** - Avoid monolithic GDAL entirely
6. **Contribute back to GeoRust** - Submit improvements and new features upstream
7. **Optimize for streaming** - Enable partial reads and cloud-optimized access

#### 4. GeoETL Execution (`geoetl-exec`)

**Responsibility**: Query planning, optimization, and execution

**Components**:

- **Logical Plan Builder**
  - Translate user operations into DataFusion logical plans
  - Represent spatial operations as plan nodes
  - Compose operations into execution graph

- **Query Optimizer**
  - Predicate pushdown to format readers (leveraging `geozero` streaming)
  - Spatial index utilization (via `rstar`)
  - Join optimization for spatial predicates
  - Partition pruning for distributed execution

- **Physical Execution**
  - Vectorized execution using Arrow
  - Parallel processing with thread pools
  - Memory management and spilling
  - Progress tracking

#### 5. GeoETL Ballista (`geoetl-ballista`)

**Responsibility**: Distributed execution integration

**Components**:

- **Cluster Management**
  - Scheduler deployment and configuration
  - Executor registration and health monitoring
  - Work distribution and load balancing

- **Distributed Operations**
  - Partition-aware spatial operations
  - Distributed spatial joins
  - Shuffle operations for repartitioning
  - Fault tolerance and recovery

- **Storage Integration**
  - Object storage readers via `object_store` crate (S3, Azure Blob, GCS)
  - Distributed caching
  - Data locality optimization

### Data Flow

#### Single-Node Processing

```
Input File → Format Reader (GeoRust/Custom) → Arrow RecordBatch →
Spatial Operations (geo crate) → DataFusion Processing →
Format Writer (geozero) → Output File
```

#### Distributed Processing

```
Input Data (Cloud Storage via object_store) → Ballista Scheduler →
Task Distribution → Executor Nodes →
Parallel Processing → Shuffle/Combine →
Result Aggregation → Output (Cloud Storage)
```

### Memory Model

**Arrow Columnar Format**:
- All data processing uses Apache Arrow in-memory format
- Vectorized operations on columnar data
- Zero-copy reads where possible (leveraging `geozero`)
- Efficient serialization for network transfer

**Memory Management**:
- Rust's ownership system prevents memory leaks
- Streaming processing for large datasets via `geozero`
- Configurable memory limits and spilling to disk
- Reference counting for shared data (Arc)

### Concurrency Model

**Single-Node**:
- Tokio async runtime for I/O operations
- Rayon thread pool for CPU-intensive operations
- Lock-free data structures where possible
- Work stealing for load balancing

**Distributed**:
- Ballista scheduler coordinates work
- gRPC for inter-node communication
- Arrow Flight for efficient data transfer
- Partition-based parallelism

## Technology Stack

### Core Technologies

| Component | Technology | Justification |
|-----------|-----------|---------------|
| Language | Rust 1.90+ | Memory safety, performance, modern tooling |
| Query Engine | Apache DataFusion | Proven vectorized execution, extensible |
| Distributed | DataFusion Ballista | Built on DataFusion, Arrow-native |
| Data Format | Apache Arrow | Industry standard for analytics |
| CLI Framework | clap 4.x | Robust argument parsing, derive macros |
| Async Runtime | Tokio | De facto standard for async Rust |
| Parallelism | Rayon | Easy data parallelism, work stealing |
| Serialization | Parquet, Arrow IPC | Efficient columnar storage |

### GeoRust Ecosystem Libraries

| Component | Crate | Purpose |
|-----------|-------|---------|
| Core Geometry | `geo` | Geometric algorithms and operations |
| Geometry Types | `geo-types` | Common geometric type definitions |
| Streaming I/O | `geozero` | Zero-copy geospatial data streaming |
| GeoJSON | `geojson` | GeoJSON format support |
| FlatGeobuf | `flatgeobuf` | FlatGeobuf format support |
| CRS Transforms | `proj` | PROJ library bindings |
| WKT | `wkt` | Well-Known Text parsing |
| Spatial Index | `rstar` | R-tree spatial indexing |
| GeoHash | `geohash` | Geohash encoding/decoding |

### Additional Libraries

| Component | Crate | Purpose |
|-----------|-------|---------|
| Cloud Storage | `object_store` | Unified cloud storage interface |
| Parquet | `parquet` | Parquet columnar format |
| SQLite | `rusqlite` | SQLite (for GeoPackage) |
| Compression | `flate2`, `zstd` | Compression algorithms |

## Design Patterns

### 1. Plugin Architecture with GeoZero

```rust
use geozero::GeozeroGeometry;

trait FormatReader: Send + Sync {
    fn read(&self, path: &str) -> Result<RecordBatch>;
    fn schema(&self) -> Result<Schema>;
    fn supports_streaming(&self) -> bool;
}

// Formats register themselves
FormatRegistry::register("geojson", GeoJsonReader::new());
```

### 2. Builder Pattern for Operations

```rust
use geo::algorithm::Buffer;

SpatialOperation::new()
    .input(source)
    .buffer(distance)
    .simplify(tolerance)
    .project(target_crs)
    .execute()?
```

### 3. Extension Traits for DataFusion

```rust
use geo::Geometry;

trait SpatialDataFrameExt {
    fn spatial_filter(&self, geometry: Geometry) -> DataFrame;
    fn buffer(&self, distance: f64) -> DataFrame;
    fn spatial_join(&self, other: &DataFrame) -> DataFrame;
}

impl SpatialDataFrameExt for DataFrame { ... }
```

### 4. Error Handling

```rust
use thiserror::Error;

#[derive(Debug, Error)]
enum GeoError {
    #[error("Format not supported: {0}")]
    UnsupportedFormat(String),

    #[error("CRS transformation failed: {0}")]
    CrsError(#[from] proj::ProjError),

    #[error("Geometric operation failed: {0}")]
    GeometryError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}
```

## Performance Targets

### Benchmarks

| Operation | Target vs GDAL | Method |
|-----------|---------------|---------|
| Format conversion (GeoJSON → Parquet) | 5-10x faster | `geozero` streaming, columnar output |
| Spatial filter (10M features) | 5x faster | `rstar` indexing, SIMD |
| Buffer operation (1M polygons) | 3-5x faster | Parallel `geo` algorithms |
| Spatial joins (large datasets) | 5x faster | Partition-based parallelism |
| Distributed processing (1TB) | Linear scaling | Ballista partition-based parallelism |

### Optimization Strategies

1. **Vectorization**: Use SIMD for numeric operations
2. **Parallelism**: Multi-threaded by default via Rayon
3. **Streaming**: Process data in chunks using `geozero`
4. **Indexing**: Use `rstar` R-tree for spatial queries
5. **Predicate Pushdown**: Filter at read time when format supports it
6. **Zero-Copy**: Minimize data copying using Arrow slicing and `geozero`
7. **Lazy Execution**: Build execution plan before processing
8. **Partition Pruning**: Skip unnecessary data partitions

## GeoRust Ecosystem Integration

### Contributing Back

We commit to:
1. **Report issues** in GeoRust crates as we encounter them
2. **Submit bug fixes** and performance improvements upstream
3. **Contribute new features** needed for GeoETL back to GeoRust
4. **Share benchmarks** to help improve ecosystem performance
5. **Collaborate** with GeoRust maintainers on API design

### Planned Contributions

- Performance optimizations for `geo` algorithms
- Cloud-optimized readers for `geozero`
- GeoParquet extensions for `parquet` crate
- Distributed spatial join implementations
- Benchmarking infrastructure

## Extensibility

### Custom Formats

Users can add custom format support:

```rust
use geozero::GeozeroGeometry;

struct MyFormatReader { ... }

impl FormatReader for MyFormatReader {
    fn read(&self, path: &str) -> Result<RecordBatch> {
        // Custom implementation using geozero
    }
}

// Register at runtime
geoetl::register_format("myformat", MyFormatReader::new());
```

### Custom Operations

Users can define custom spatial operations:

```rust
use geo::Geometry;

#[derive(Debug)]
struct CustomOperation { ... }

impl SpatialOp for CustomOperation {
    fn execute(&self, input: RecordBatch) -> Result<RecordBatch> {
        // Custom logic using geo crate
    }
}
```

### DataFusion UDFs

Leverage DataFusion's UDF system:

```rust
use datafusion::logical_expr::create_udf;

let st_buffer = create_udf(
    "st_buffer",
    vec![DataType::Binary, DataType::Float64],
    Arc::new(DataType::Binary),
    Volatility::Immutable,
    Arc::new(buffer_impl),
);

ctx.register_udf(st_buffer);
```

## Security Considerations

1. **Memory Safety**: Rust prevents buffer overflows, use-after-free
2. **Input Validation**: Validate all file inputs for malformed data
3. **Resource Limits**: Enforce memory and CPU limits
4. **Sandboxing**: Consider sandboxing for user-defined functions
5. **Dependency Auditing**: Regular security audits (`cargo audit`)
6. **Cloud Credentials**: Secure handling via `object_store` credential providers
7. **Minimal C Dependencies**: Limited attack surface from C bindings

## Testing Strategy

### Unit Tests
- Per-module tests in Rust standard fashion
- Property-based testing with `proptest`
- Geometry correctness tests against `geo` test suite

### Integration Tests
- End-to-end CLI command tests
- Format round-trip tests (read → write → read)
- DataFusion query plan validation
- GeoZero streaming tests

### Performance Tests
- Benchmark suite using `criterion`
- Regression testing against baseline
- Comparison benchmarks vs GDAL
- `geo` algorithm performance profiling

### Compatibility Tests
- Validate output against reference implementations
- Cross-format conversion correctness
- CRS transformation accuracy (via `proj`)
- OGC Simple Features compliance

## Migration Path

### Phase 1: Foundation (Q1 2026)
- Basic workspace structure
- CLI scaffolding with `clap`
- DataFusion integration
- GeoJSON support via `geojson` crate
- GeoParquet support with custom extensions
- Streaming I/O via `geozero`

### Phase 2: Core Features (Q2 2026)
- Additional vector formats (`flatgeobuf`, GeoPackage)
- Core spatial operations using `geo` crate
- CRS support via `proj` crate
- Spatial indexing with `rstar`
- Performance benchmarking vs GDAL

### Phase 3: Advanced Features (Q3 2026)
- Advanced spatial algorithms and optimizations
- Complex spatial joins
- Query optimization passes
- Performance parity with GDAL for vector operations

### Phase 4: Distribution (Q4 2026)
- Ballista integration
- Distributed operations
- Cloud storage via `object_store`
- Horizontal scaling tests

## Consequences

### Positive

1. **Performance**: Significant speedups from vectorization and parallelism
2. **Safety**: Rust eliminates entire classes of bugs
3. **Scalability**: Seamless single-node to distributed scaling
4. **Maintainability**: Modern codebase easier to understand and extend
5. **Ecosystem**: Leverages and strengthens GeoRust ecosystem
6. **Cloud Native**: First-class cloud storage via `object_store`
7. **No GDAL Dependency**: Simpler build, smaller footprint
8. **Community**: Contributes to growing Rust geospatial community

### Negative

1. **Learning Curve**: Rust has steeper learning curve than C++ or Python
2. **Custom Implementations**: Need to write formatters for some formats
3. **Vector-Only Focus**: Does not support raster data processing
4. **Compilation Time**: Rust compilation slower than interpreted languages
5. **Initial Format Coverage**: May support fewer formats than GDAL initially

### Risks and Mitigations

| Risk | Impact | Likelihood | Mitigation |
|------|--------|-----------|------------|
| DataFusion performance doesn't meet targets | High | Medium | Benchmark early, contribute optimizations upstream |
| Format compatibility issues | High | Medium | Extensive testing, contribute to GeoRust |
| Missing GeoRust features | Medium | Medium | Contribute features back to GeoRust |
| Ballista adoption/stability | Medium | Low | Start single-node, add distributed incrementally |
| Limited Rust geospatial talent | Medium | Medium | Documentation, mentorship, community building |
| Custom format complexity | Medium | Low | Prioritize common formats, reference specs |

## References

### Core Technologies
- [Apache DataFusion Documentation](https://arrow.apache.org/datafusion/)
- [DataFusion Ballista](https://github.com/apache/arrow-datafusion/tree/main/ballista)
- [Apache Arrow Format Specification](https://arrow.apache.org/docs/format/Columnar.html)
- [The Rust Programming Language](https://doc.rust-lang.org/book/)

### GeoRust Ecosystem
- [GeoRust Organization](https://github.com/georust)
- [geo crate](https://docs.rs/geo/)
- [geozero crate](https://docs.rs/geozero/)
- [proj crate](https://docs.rs/proj/)
- [rstar crate](https://docs.rs/rstar/)

### Specifications
- [GeoParquet Specification](https://geoparquet.org/)
- [GeoJSON Specification (RFC 7946)](https://tools.ietf.org/html/rfc7946)
- [FlatGeobuf Specification](https://flatgeobuf.org/)
- [OGC Simple Features](https://www.ogc.org/standards/sfa)

### Reference
- [GDAL Documentation](https://gdal.org/) (for compatibility reference only)

## Decision Log

- **2025-01**: Initial ADR created
- **2025-01**: Rust + DataFusion + Ballista + GeoRust architecture approved
- **2025-01**: Decided to avoid GDAL dependency entirely
- **2025-01**: Committed to strongly leveraging and contributing to GeoRust

---

**Next ADRs to Consider**:
- ADR 0002: Vector Format Support Strategy and Implementation Priorities
- ADR 0003: Error Handling and Observability
- ADR 0004: Plugin Architecture Design
- ADR 0005: GeoZero Integration Strategy
- ADR 0006: Spatial Indexing with RTree
- ADR 0007: Distributed Spatial Operations
