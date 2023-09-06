import fetch from "cross-fetch";

// These are added so that the WASM code can use fetch
export default function polyfills() {
  // @ts-ignore
  global.fetch = fetch;
  // @ts-ignore
  global.Headers = fetch.Headers;
  // @ts-ignore
  global.Request = fetch.Request;
  // @ts-ignore
  global.Response = fetch.Response;
}
