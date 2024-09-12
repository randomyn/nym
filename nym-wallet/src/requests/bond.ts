import {
  Fee,
  TransactionExecuteResult,
  MixNodeCostParams,
  GatewayBond,
  NymNodeDetails,
  MixNodeDetails,
} from '@nymproject/types';
import { TBondMixNodeArgs, TBondMixnodeSignatureArgs, EnumNodeType, TUpdateBondArgs } from 'src/types';
import { invokeWrapper } from './wrapper';

export const unbondGateway = async (fee?: Fee) => invokeWrapper<TransactionExecuteResult>('unbond_gateway', { fee });

export const bondMixNode = async (args: TBondMixNodeArgs) =>
  invokeWrapper<TransactionExecuteResult>('bond_mixnode', args);

export const generateMixnodeMsgPayload = async (args: Omit<TBondMixnodeSignatureArgs, 'tokenPool'>) =>
  invokeWrapper<string>('generate_mixnode_bonding_msg_payload', args);

export const unbondMixNode = async (fee?: Fee) => invokeWrapper<TransactionExecuteResult>('unbond_mixnode', { fee });

export const updateMixnodeCostParams = async (newCosts: MixNodeCostParams, fee?: Fee) =>
  invokeWrapper<TransactionExecuteResult>('update_mixnode_cost_params', { newCosts, fee });

export const unbond = async (type: EnumNodeType) => {
  if (type === EnumNodeType.mixnode) return unbondMixNode();
  return unbondGateway();
};

export const updateBond = async (args: TUpdateBondArgs) =>
  invokeWrapper<TransactionExecuteResult>('update_pledge', args);

export const getNymNodeBondDetails = async () => invokeWrapper<NymNodeDetails | null>('nym_node_bond_details');

export const getMixnodeBondDetails = async () => invokeWrapper<MixNodeDetails | null>('mixnode_bond_details');

export const getGatewayBondDetails = async () => invokeWrapper<GatewayBond | null>('gateway_bond_details');
