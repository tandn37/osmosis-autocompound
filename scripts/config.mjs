import dotenv from 'dotenv';
dotenv.config();

export default {
  chainName: 'osmosis',
  chainPrefix: 'osmo',
  defaultGasPrice: '0.025uosmo',
  // rpcEndpoint: 'http://localhost:26657/',
  rpcEndpoint: 'https://rpc-test.osmosis.zone',
  mnemonic: process.env.MNEMONIC,
}