# Permissions Hub Smart Contract

## Overview
The Permissions Hub is a security-focused smart contract that manages permissions for on-behalf operations across the MultiversX DeFi ecosystem. It allows users to whitelist specific contracts that can perform operations on their behalf, enabling secure contract-to-contract interactions while maintaining user control over their assets.

## Features
- User-controlled whitelisting of trusted contracts
- Administrative blacklisting for security purposes
- Granular permission management
- Efficient permission checking through optimized storage
- Integration support for other smart contracts

## Core Functionality

### User Operations

#### Whitelisting Contracts
Users can whitelist multiple contract addresses that they trust to operate on their behalf:
```rust
#[endpoint]
fn whitelist(&self, addresses_to_whitelist: MultiValueEncoded<ManagedAddress>)
```
- Allows users to add multiple trusted contracts in a single transaction
- Prevents duplicate whitelisting through built-in validation
- Each user maintains their own whitelist independently

#### Removing Whitelisted Contracts
Users can remove previously whitelisted contracts:
```rust
#[endpoint(removeWhitelist)]
fn remove_whitelist(&self, addresses_to_remove: MultiValueEncoded<ManagedAddress>)
```
- Allows batch removal of whitelisted addresses
- Validates that addresses were previously whitelisted
- Maintains user control over their permissions

### Administrative Functions

#### Blacklisting
Contract owner can blacklist potentially malicious addresses:
```rust
#[only_owner]
#[endpoint(blacklist)]
fn blacklist(&self, address_to_blacklist: ManagedAddress)
```
- Restricted to contract owner
- Global blacklist affecting all users
- Security measure against identified threats

#### Removing from Blacklist
Contract owner can remove addresses from the blacklist:
```rust
#[only_owner]
#[endpoint(removeBlacklist)]
fn remove_blacklist(&self, address_to_remove: ManagedAddress)
```

### View Functions

#### Permission Checking
Contracts can verify if they have permission to operate on behalf of a user:
```rust
#[view(isWhitelisted)]
fn is_whitelisted(&self, user: &ManagedAddress, address_to_check: &ManagedAddress) -> bool
```
- Returns true only if:
  1. The address is not blacklisted
  2. The address is in the user's whitelist
- Efficient for integration with other contracts

#### Blacklist Viewing
```rust
#[view(getBlacklistedAddresses)]
fn blacklisted_addresses(&self) -> UnorderedSetMapper<ManagedAddress>
```
- Public view of globally blacklisted addresses
- Useful for transparency and integration purposes

## Storage

The contract uses two main storage mappers:

1. User Whitelists:
```rust
#[storage_mapper("whitelistedAddresses")]
fn user_whitelisted_addresses(&self, user: &ManagedAddress) -> UnorderedSetMapper<ManagedAddress>
```
- Separate whitelist for each user
- Implemented as an UnorderedSetMapper for efficient operations

2. Global Blacklist:
```rust
#[storage_mapper("blacklistedAddresses")]
fn blacklisted_addresses(&self) -> UnorderedSetMapper<ManagedAddress>
```
- Single global blacklist
- Managed by contract owner

## Integration Guide

### For Smart Contracts
To integrate with the Permissions Hub:

1. Add the Permissions Hub address as a configurable parameter in your contract
2. Before performing operations on behalf of a user, check permissions:
```rust
let is_allowed = permissions_hub_proxy.is_whitelisted(user_address, caller_address);
require!(is_allowed, "Not authorized to perform operations on behalf of user");
```

### For Users
To enable contracts to operate on your behalf:

1. Call the `whitelist` endpoint with the contract address(es) you want to authorize
2. Monitor your active whitelisted addresses
3. Remove permissions using `removeWhitelist` when they're no longer needed

## Security Considerations

- Users should carefully verify contract addresses before whitelisting
- Regular auditing of whitelisted addresses is recommended
- The blacklist provides an additional security layer managed by the contract owner
- All permission changes are permanent until explicitly modified
- Users maintain full control over their whitelist
