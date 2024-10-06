import { getSigningOsmosisClient } from 'osmojs';
import { chains } from 'chain-registry';
import { getOfflineSignerProto as getOfflineSigner } from 'cosmjs-utils';
import { CosmWasmClient, SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate';

import ProtoSigning from "@cosmjs/proto-signing";
import Stargate from "@cosmjs/stargate";

import config from './config.mjs';

const getLockWallets = (client) => (lastRequest) => {

}

const getRPCClient = async () => {
  const chain = chains.find(({ chain_name }) => chain_name === config.chainName);
  const signer = await getOfflineSigner({
    mnemonic: config.mnemonic,
    chain,
  });
  const client = await getSigningOsmosisClient({
    rpcEndpoint: config.rpcEndpoint,
    signer,
  });
  return client;
}

const getClient2 = async () => {
  const signer = await ProtoSigning.DirectSecp256k1HdWallet.fromMnemonic(
    config.mnemonic, { prefix: config.chainPrefix },
  );
  const options = {
    prefix: config.chainPrefix,
    gasPrice: Stargate.GasPrice.fromString(config.defaultGasPrice)
  };
  const client = await SigningCosmWasmClient.connectWithSigner(config.rpcEndpoint, signer, options);
  return client;
}

const main = async () => {
  console.log('Starting');
  const client = await getClient2();
  console.log('client', client);
  client.queryContractSmart
  const result =await client.getAllBalances('osmo1wclew8t48cac0aecxcxdrt2ynw8gt8ad9u6axe')
  console.log('result', result);
}

main();