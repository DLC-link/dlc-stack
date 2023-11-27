# dlc-storage

The `dlc-storage` project is a Rust framework for providing storage operations for the oracle / wallet.

## TODOs

- It has one API, but would be wise to separate the reader and writer to use different APIs
- Use caching for the reader
- Clean the cache (or proper caches) ones a write happens by the writer
- Separate common module - contract and event tables should be not part of the migration together, ideally they should be used in different databases
- Oracles should neever use the same DB, but if that would be the case create a new field for identifying the oracle for the events
- Currently it works only with postgres - extend this if needed

## License

The `dlc-storage` project is licensed under the [APM 2.0 license](LICENSE).
