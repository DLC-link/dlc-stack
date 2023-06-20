import * as ethers from 'ethers';
import { TransactionResponse } from '@ethersproject/abstract-provider';
import { MintEthereum } from '../models/DTOs/mint-eth.dto';
import { NFTStorage, File } from 'nft.storage';
import Jimp from 'jimp';
// The 'mime' npm package helps us set the correct file type on our File objects
import mime from 'mime-types';

// The 'fs' builtin module on Node.js provides access to the file system
import fs from 'fs';

// The 'path' module provides helpers for manipulating filesystem paths
import path from 'path';

export async function mintNft(params: MintEthereum, nftContractWithSigner: ethers.Contract) {
  console.log('params into mint function', params);

  const nftCount: number = await nftContractWithSigner.getNextMintId();
  console.log('got nft count as ' + nftCount); // using this to set the nft.png name

  const NFT_STORAGE_TOKEN = process.env.NFT_STORAGE_TOKEN as string;
  const client = new NFTStorage({ token: NFT_STORAGE_TOKEN });

  const imageNumber = nftCount % 72; // 72 is the number of unique pngs created so far, 0 - 71. 72 % 72 = 0.
  const nftImage = await Jimp.read(`./btcNftImages/${imageNumber}.png`);
  const font = await Jimp.loadFont(Jimp.FONT_SANS_32_WHITE);
  const satsAmount = params.collateral.toLocaleString('en-US');
  nftImage.print(
    font,
    0,
    0,
    {
      text: satsAmount + ' sats',
      alignmentX: Jimp.HORIZONTAL_ALIGN_RIGHT,
      alignmentY: Jimp.VERTICAL_ALIGN_BOTTOM,
    },
    512
  );
  await nftImage.writeAsync(`./nft/${imageNumber}.png`);

  const nftImageWithText = await fileFromPath(`./nft/${imageNumber}.png`, `/nft/${imageNumber}.png`);

  const metadata = await client.store({
    name: 'Native Bitcoin backed NFT',
    description: `This is an NFT which represents ${
      params.collateral / 10 ** 8
    } of locked Bitcoin -- https://www.dlc.link`,
    image: nftImageWithText,
    // properties: {
    //     custom: 'Custom data can appear here, files are auto uploaded.',
    //     file: new File(['<DATA>'], 'README.md', { type: 'text/plain' }),
    // }
  });
  console.log(`IPFS URL for the metadata: `, metadata.url);
  console.log(`metadata.json contents:\n`, metadata.data);
  console.log(`metadata.json with IPFS gateway URLs`, metadata.embed());

  console.log(
    `Minting NFT \n receiver: ${params.receiver},\n url: ${metadata.url.substring(7)}, \n creator: ${
      params.creator
    }, \n dlcUUID: ${params.uuid}`
  );

  // function safeMint(address to, string memory uri, address broker, bytes32 dlcUUID)
  const tx: TransactionResponse = await nftContractWithSigner.safeMint(
    params.receiver,
    metadata.url.substring(7),
    params.creator,
    params.uuid,
    {
      gasLimit: 500000,
      nonce: undefined,
    }
  );
  await tx.wait();
  return nftCount;
}

/**
 * A helper to read a file from a location on disk and return a File object.
 * Note that this reads the entire file into memory and should not be used for
 * very large files.
 * @param {string} filePath the path to a file to store
 * @returns {File} a File object containing the file content
 */
async function fileFromPath(filePath: string, nameToUse: string) {
  const content = await fs.promises.readFile(filePath);
  const type = mime.lookup(filePath);
  return new File([content], path.basename(nameToUse), {
    type: type || undefined,
  });
}
