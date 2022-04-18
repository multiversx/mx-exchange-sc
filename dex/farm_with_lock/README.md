# Farm With Lock Smart Contract

## Abstract

This contract is very similar with the regular Farm contract. It is recommended that you go though its README first, as this doc will cover only the differences.

## Introduction

This contract will rewards its participants with Locked MEX instead of MEX. Locked MEX is a META-ESDT with certain metadata stored as attributes. Because the META-ESDT create role can be held by only one address per shard, instead of creating the Locked MEX itself, the contract will do a request to Locked MEX Factory to create and forward the tokens to the user, on its behalf.

## Endpoints

The same as Farm contract.

## Testing

The same as Farm contract.

## Interaction

The same as Farm contract.

## Deployment

The same as Farm contract.
