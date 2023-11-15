# dlc-storage

The `dlc-storage` project is a Rust framework for providing storage operations for the attestors / wallets.

# security
The client to the API uses their private key to sign the JSON request parameters +
a nonce retrieved from the API server. Then this signed package is sent to the API server
along with the clear JSON parameters and the pub key. The API server runs an
ECDSA.verify to ensure that the pub key of the client can verify that the message is
valid for that pubkey and has not been tampered with. If so, it process the request
and deletes the nonce from it's set of valid nonces.

## TODOs

- It has one API, but would be wise to separate the reader and writer to use different APIs
- Use caching for the reader
- Clean the cache (or proper caches) once a write happens by the writer
- Separate common module - contract and event tables should be not part of the migration together, ideally they should be used in different databases

## License

The `dlc-storage` project is licensed under the [APM 2.0 license](LICENSE).
