# Changelog

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/). This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## \[Unreleased\]

## 0.0.65

## 0.0.64

## 0.0.63

## 0.0.62

## 0.0.61

## 0.0.60

## 0.0.59

- Adds `--recursive` command to `hc web-app pack` and `hc app pack` which packs all bundled dependencies for the given manifest. So `hc app pack ./workdir --recursive` will first go to each of the DNA manifests which have their location specified as bundled in the app manifest, pack each of them, and finally pack the app itself. `hc web-app pack ./workdir --recursive` will first pack the app recursively first if specified as bundled, and then pack the web-app manifest itself.

## 0.0.58

- Adds experimental `--raw` command to hc unpack commands (e.g. `hc dna unpack`) which allows an invalid manifest to still be unpacked. This can help to “salvage” a bundle which is no longer compatible with the current Holochain version, correcting the manifest so that it can be re-packed into a valid bundle.

## 0.0.57

## 0.0.56

## 0.0.55

## 0.0.54

## 0.0.53

## 0.0.52

## 0.0.51

## 0.0.50

## 0.0.49

## 0.0.48

## 0.0.47

## 0.0.46

## 0.0.45

- BREAKING CHANGE - Refactor: Property `integrity.uid` of DNA Yaml files renamed to `integrity.network_seed`. Functionality has not changed. [\#1493](https://github.com/holochain/holochain/pull/1493)

## 0.0.44

## 0.0.43

## 0.0.42

## 0.0.41

## 0.0.40

## 0.0.39

## 0.0.38

## 0.0.37

## 0.0.36

## 0.0.35

## 0.0.34

## 0.0.33

## 0.0.32

## 0.0.31

## 0.0.30

## 0.0.29

## 0.0.28

## 0.0.27

## 0.0.26

## 0.0.25

## 0.0.24

## 0.0.23

- The DNA manifest now requires an `origin_time` Timestamp field, which will be used in the forthcoming gossip optimization.
  - There is a new system validation rule that all Header timestamps (including the initial Dna header) must come after the DNA’s `origin_time` field.
  - `hc dna init` injects the current system time as *microseconds* for the `origin_time` field of the DNA manifest
  - Since this field is not actually hooked up to anything at the moment, if the field is not present in a DNA manifest, a default `origin_time` of `January 1, 2022 12:00:00 AM` will be used instead. Once the new gossip algorithm lands, this default will be removed, and this will become a breaking change for DNA manifests which have not yet added an `origin_time`.

## 0.0.22

## 0.0.21

## 0.0.20

## 0.0.19

## 0.0.18

## 0.0.17

## 0.0.16

## 0.0.15

## 0.0.14

## 0.0.13

## 0.0.12

## 0.0.11

## 0.0.10

## 0.0.9

## 0.0.8

## 0.0.7

## 0.0.6

## 0.0.5

- Added the `hc web-app` subcommand, with the exact same behaviour and functionality as `hc dna` and `hc app`.

## 0.0.4

## 0.0.3

## 0.0.2

## 0.0.1