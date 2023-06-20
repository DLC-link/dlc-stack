import fetch from 'cross-fetch';
import { ETHSubchain } from '../../chain-types.interface';
import { DeploymentInfo } from '../models/interfaces/deployment-info.interface';
import fs from 'fs';

export async function fetchDeployment(
  contract: string,
  subchain: ETHSubchain,
  localhost?: boolean
): Promise<DeploymentInfo> {
  // If we are running the app locally we need to fetch the deployment files from the local directory
  if (localhost) {
    let dp = JSON.parse(fs.readFileSync(`./deploymentFiles/localhost/${contract}.json`, 'utf-8'));
    return dp;
  }

  // TODO: sometimes we might not want to read from the mastr branch.....
  const response = await fetch(
    `https://raw.githubusercontent.com/DLC-link/dlc-solidity/master/deploymentFiles/${subchain}/${contract}.json`
  );
  return (await response.json()) as DeploymentInfo;
}
