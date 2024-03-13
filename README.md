# Molnctl

Molnctl is the commandline interface to interact with [Molnett](molnett.com).

## Installation

### Download from Github

We recommend downloading the latest version from the [releases page](https://github.com/molnett/molnctl/releases).
There are pre-compiled versions available for MacOS, Linux and Windows.

### Compile it yourself

If you want to compile it yourself, you need a working Rust environment. We recommend following the [official docs](https://www.rust-lang.org/tools/install).
Once you have Rust setup, a simple `cargo build --release` should produce your very own molnctl.

## Usage

### Setup

Here's what you need to get started.
```
molnctl auth login
molnctl orgs switch
```

### Getting help

The built in help will guide you through the various commands available.
```
molnctl -h
```
