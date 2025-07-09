# Test Directory Structure

This directory contains all test files and fixtures for the molnctl project.

## Directory Structure

```
tests/
├── README.md                       # This file - documentation
├── BUILDKIT_EVENTS_ANALYSIS.md    # BuildKit event analysis documentation
├── integration_build_tests.rs     # Integration tests for build functionality
├── build_stats_tests.rs           # Tests for build statistics and caching
└── fixtures/                      # Test fixtures and sample files
    ├── dockerfiles/               # Sample Dockerfiles for testing
    │   ├── stats-test.Dockerfile  # Python app for testing build stats
    │   ├── complex-test.Dockerfile # Complex multi-stage build
    │   └── final-test.Dockerfile  # Final test scenario
    └── apps/                      # Sample application files
        ├── app.py                 # Simple Python Flask app
        ├── requirements.txt       # Python dependencies
        └── package.json          # Node.js package file
```

## Test Categories

### Integration Tests (`integration_build_tests.rs`)
- Basic build functionality
- Docker ignore file handling
- Platform-specific builds
- Missing Dockerfile validation
- Build context creation

### Build Statistics Tests (`build_stats_tests.rs`)
- Build statistics accuracy
- Cache hit/miss tracking
- Layer counting consistency
- Base image layer detection

All tests call the built binary using `env!("CARGO_BIN_EXE_molnctl")` rather than accessing internal modules directly, making them true integration tests.

## Test Fixtures

### Dockerfiles
- `stats-test.Dockerfile`: Multi-stage Python application for testing build statistics
- `complex-test.Dockerfile`: Complex multi-stage build scenario
- `final-test.Dockerfile`: Final comprehensive test case

### Sample Applications
- `app.py` + `requirements.txt`: Simple Python Flask application
- `package.json`: Node.js application metadata

## Running Tests

```bash
# Run all tests
cargo test

# Run only integration tests
cargo test integration_build_tests

# Run only build statistics tests
cargo test build_stats_tests

# Run tests with output
cargo test -- --nocapture
```

## Test Requirements

- Docker must be running and accessible
- Git repository must be initialized (for commit SHA generation)
- Network access may be required for pulling base images

## Adding New Tests

1. Create test functions in appropriate test files
2. Add new fixtures to the `fixtures/` directory as needed
3. Update this README if new test categories are added
4. Ensure tests clean up Docker images they create

## Debugging Tests

To debug failing tests:

1. Run with verbose output: `cargo test -- --nocapture`
2. Check Docker daemon status: `docker info`
3. Inspect test fixtures in `tests/fixtures/`
4. Review build output in test failure messages