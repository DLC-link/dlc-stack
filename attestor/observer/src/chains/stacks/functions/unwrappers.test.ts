import { describe, expect, test } from '@jest/globals';
import type { ContractCallTransaction } from '@stacks/stacks-blockchain-api-types';
import unwrapper, { unwrapEventSource, unwrapPrintEvents } from './unwrappers';

test('unwrapEventSource correctly unwraps values', () => {
  const testCases = [
    {
      input: 'event:functionName:source',
      expectedOutput: { event: 'functionName', source: 'source' },
    },
    {
      input: 'event:anotherFunction:anotherSource',
      expectedOutput: { event: 'anotherFunction', source: 'anotherSource' },
    },
  ];

  testCases.forEach((testCase) => {
    const { input, expectedOutput } = testCase;
    const result = unwrapEventSource(input);
    expect(result).toEqual(expectedOutput);
  });
});

const tx: ContractCallTransaction = {
  tx_id: '0x6a83ab7fc412427ea899d907ae4e3735e95e552fbc2f4b336110dd8278eb33f0',
  nonce: 24,
  fee_rate: '100000',
  sender_address: 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM',
  sponsored: false,
  post_condition_mode: 'deny',
  post_conditions: [],
  anchor_mode: 'on_chain_only',
  is_unanchored: false,
  block_hash: '0x74d5d70a58774e68e3a9233083a1d67eb20f58e6b124ac85307ca8b31f91cb85',
  parent_block_hash: '0xf0472239dc5cbc23a509741efb4cbaacd9df87aa4f599569a6bc890639b59eed',
  block_height: 127,
  burn_block_time: 1671011861,
  burn_block_time_iso: '2022-12-14T09:57:41.000Z',
  parent_burn_block_time: 1671011851,
  parent_burn_block_time_iso: '2022-12-14T09:57:31.000Z',
  canonical: true,
  tx_index: 1,
  tx_status: 'success',
  tx_result: {
    hex: '0x0703',
    repr: '(ok true)',
  },
  microblock_hash: '0x',
  microblock_sequence: 2147483647,
  microblock_canonical: true,
  event_count: 3,
  events: [
    {
      event_index: 0,
      event_type: 'smart_contract_log',
      tx_id: '0x6a83ab7fc412427ea899d907ae4e3735e95e552fbc2f4b336110dd8278eb33f0',
      contract_log: {
        contract_id: 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.dlc-manager-priced-v0-1',
        topic: 'print',
        value: {
          hex: '0x0c000000030c6576656e742d736f757263650d00000020646c636c696e6b3a76616c69646174652d70726963652d646174613a76302d310570726963650100000000000000000000019e7bae96e004757569640200000020a706e0be4bd16e81673201b6314d632d9f0a4681ebef9a641716e65623fda047',
          repr: '(tuple (event-source "dlclink:validate-price-data:v0-1") (price u1780191500000) (uuid 0xa706e0be4bd16e81673201b6314d632d9f0a4681ebef9a641716e65623fda047))',
        },
      },
    },
    {
      event_index: 1,
      event_type: 'smart_contract_log',
      tx_id: '0x6a83ab7fc412427ea899d907ae4e3735e95e552fbc2f4b336110dd8278eb33f0',
      contract_log: {
        contract_id: 'STNHKEPYEPJ8ET55ZZ0M5A34J0R3N5FM2CMMMAZ6.sample-contract-loan-v0-1',
        topic: 'print',
        value: {
          hex: '0x0c00000002067374617475730d0000000e7072652d6c69717569646174656404757569640200000020a706e0be4bd16e81673201b6314d632d9f0a4681ebef9a641716e65623fda047',
          repr: '(tuple (status "pre-liquidated") (uuid 0xa706e0be4bd16e81673201b6314d632d9f0a4681ebef9a641716e65623fda047))',
        },
      },
    },
    {
      event_index: 2,
      event_type: 'smart_contract_log',
      tx_id: '0x6a83ab7fc412427ea899d907ae4e3735e95e552fbc2f4b336110dd8278eb33f0',
      contract_log: {
        contract_id: 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.dlc-manager-priced-v0-1',
        topic: 'print',
        value: {
          hex: '0x0c000000061163616c6c6261636b2d636f6e7472616374061a2b19bade75a48768a5ffc142a86490303a95f4131973616d706c652d636f6e74726163742d6c6f616e2d76302d310663616c6c6572051a6d78de7b0625dfbfc16c3a8a5735f6dc3dc3f2ce0763726561746f72051a2b19bade75a48768a5ffc142a86490303a95f4130c6576656e742d736f757263650d00000016646c636c696e6b3a636c6f73652d646c633a76302d31076f7574636f6d65010000000000000000000000000586498f04757569640200000020a706e0be4bd16e81673201b6314d632d9f0a4681ebef9a641716e65623fda047',
          repr: "(tuple (callback-contract 'STNHKEPYEPJ8ET55ZZ0M5A34J0R3N5FM2CMMMAZ6.sample-contract-loan-v0-1) (caller 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM) (creator 'STNHKEPYEPJ8ET55ZZ0M5A34J0R3N5FM2CMMMAZ6) (event-source \"dlclink:close-dlc:v0-1\") (outcome u92686735) (uuid 0xa706e0be4bd16e81673201b6314d632d9f0a4681ebef9a641716e65623fda047))",
        },
      },
    },
  ],
  execution_cost_read_count: 23,
  execution_cost_read_length: 46413,
  execution_cost_runtime: 88004000,
  execution_cost_write_count: 1,
  execution_cost_write_length: 531,
  tx_type: 'contract_call',
  contract_call: {
    contract_id: 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.dlc-manager-priced-v0-1',
    function_name: 'validate-price-data',
    function_signature:
      '(define-public (validate-price-data (uuid (buff 32)) (timestamp uint) (entries (list 10 (tuple (symbol (buff 32)) (value uint)))) (signature (buff 65)) (callback-contract trait_reference)))',
    function_args: [
      {
        hex: '0x0200000020a706e0be4bd16e81673201b6314d632d9f0a4681ebef9a641716e65623fda047',
        repr: '0xa706e0be4bd16e81673201b6314d632d9f0a4681ebef9a641716e65623fda047',
        name: 'uuid',
        type: '(buff 32)',
      },
      {
        hex: '0x0100000000000000000000018510110c81',
        repr: 'u1671011830913',
        name: 'timestamp',
        type: 'uint',
      },
      {
        hex: '0x0b000000010c000000020673796d626f6c02000000034254430576616c75650100000000000000000000019e7bae96e0',
        repr: '(list (tuple (symbol 0x425443) (value u1780191500000)))',
        name: 'entries',
        type: '(list 10 (tuple (symbol (buff 32)) (value uint)))',
      },
      {
        hex: '0x0200000041cf16c89e1eef9627d8d474ca994a617e826be0a9f61b0a6d0f8649aa736c4c3434277116f2adbd57a534bf9328b7573560a91a640f3dda0c71eb253448ddcbdb01',
        repr: '0xcf16c89e1eef9627d8d474ca994a617e826be0a9f61b0a6d0f8649aa736c4c3434277116f2adbd57a534bf9328b7573560a91a640f3dda0c71eb253448ddcbdb01',
        name: 'signature',
        type: '(buff 65)',
      },
      {
        hex: '0x061a2b19bade75a48768a5ffc142a86490303a95f4131973616d706c652d636f6e74726163742d6c6f616e2d76302d31',
        repr: "'STNHKEPYEPJ8ET55ZZ0M5A34J0R3N5FM2CMMMAZ6.sample-contract-loan-v0-1",
        name: 'callback-contract',
        type: 'trait_reference',
      },
    ],
  },
};

const unwrappedEvent_1 = {
  'event-source': {
    type: '(string-ascii 32)',
    value: 'dlclink:validate-price-data:v0-1',
  },
  price: { type: 'uint', value: '1780191500000' },
  uuid: {
    type: '(buff 32)',
    value: '0xa706e0be4bd16e81673201b6314d632d9f0a4681ebef9a641716e65623fda047',
  },
};

const unwrappedEvent_2 = {
  'callback-contract': {
    type: 'principal',
    value: 'STNHKEPYEPJ8ET55ZZ0M5A34J0R3N5FM2CMMMAZ6.sample-contract-loan-v0-1',
  },
  caller: {
    type: 'principal',
    value: 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM',
  },
  creator: {
    type: 'principal',
    value: 'STNHKEPYEPJ8ET55ZZ0M5A34J0R3N5FM2CMMMAZ6',
  },
  'event-source': {
    type: '(string-ascii 22)',
    value: 'dlclink:close-dlc:v0-1',
  },
  outcome: { type: 'uint', value: '92686735' },
  uuid: {
    type: '(buff 32)',
    value: '0xa706e0be4bd16e81673201b6314d632d9f0a4681ebef9a641716e65623fda047',
  },
};

const functionNames = [
  'create-dlc',
  'post-create-dlc',
  'close-dlc',
  'post-close-dlc',
  'get-btc-price',
  'validate-price-data',
  'register-contract',
  'unregister-contract',
  'set-status-funded',
];
const eventSourceAPIVersion = 'v0-1';
const eventSources = functionNames.map((name) => `dlclink:${name}:${eventSourceAPIVersion}`);
const contractName = 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.dlc-manager-priced-v0-1';

test('unwrapPrintEvent returns a list of unwrapped events', () => {
  const result = unwrapPrintEvents(tx, eventSources, contractName);
  expect(result).toBeDefined();
  expect(result).toContainEqual(unwrappedEvent_1);
  expect(result).toContainEqual(unwrappedEvent_2);
});

test('unwrapper returns a list of objects with correct elements', () => {
  const result = unwrapper(tx, eventSources, contractName);
  expect(result).toBeDefined();

  expect(result).toEqual([
    {
      printEvent: unwrappedEvent_1,
      eventSource: { event: 'validate-price-data', source: 'v0-1' },
    },
    {
      printEvent: unwrappedEvent_2,
      eventSource: { event: 'close-dlc', source: 'v0-1' },
    },
  ]);
});
