# DEX Rust fuzz testing

## Introduction

The Rust fuzzing is a testing tool for the DEX SCs, that allows us to input random data in order to assert if the SCs behaviour is as expected. It is supposed to provide both valid and invalid data, to test not only the correct workflow, but also the robustness of all the existing validations. It is based on the Rust Testing Framework, allowing for a more visual development and a step-by-step debugging.

In this implementation we covered the following DEX SCs:
- Pair SC
- Farm SC
- Locked asset factory SC
- Price discovery SC

## Rust fuzzer structure

The current structure of the fuzzer consists of (subject to future changes):
- Fuzz data - contains the initialization logic of the fuzzer
- Fuzz SC handles - one file per SC, implements the desired testing logic for each contract
- Fuzz start - the starting point of the fuzz testing

### Fuzz data

The fuzz data file allows the fuzzer initialization logic implementation and it is the place where all the common data is written. It also contains constants and parameters that alter the testing workflow. Lastly, it contains the statistics structure.
When adding a new feature or a new SC for fuzz testing, we define here the needed constants and parameters, the initialization of the new tested contract and the specific statistics that we want to output.

Note: In order to better simulate a real blockchain, the fuzzer uses multiple users and contract instances and only one general blockchain wrapper that stores all the data as the fuzzing progresses. Also, there is only one instance of a Random Number Generator (RNG), in order to be able to have predeterministic fuzzing scenarios.

### Fuzz SC handles

After setting up the fuzzer, we then need an individual file for each SC. Here, we define functions for each endpoint of the contract that we want to test. We use the blockchain wrapper to call specific endpoints, we assess if the output data is correct and we log the statistics accordingly.

Let's take for example the Pair SC. The workflow would be as follows:
- We use the RNG to choose a specific user and the pair that he will swap.
- We check for his specific balance before the swap and we choose a specific amount to swap
- We perform the swap, again, randomly selecting the direction (input or output swap)
- We check the final balance of the user and we compare it with the initial balance
- We finally log the results, whether it is a successful or a failed swap (hit or miss)

### Fuzz start

The last file we need is the one where the fuzz testing starts. Here, we initialize the fuzzer data, we layout all the specific functions that we want to test and we run the fuzz testing for a preset number of times. Also, we implement a custom logic to enforce blockchain time passing that does not necessarily reflect an accurate time passing in a real blockchain. This is for specific functions that rely on time frames. Finally, we print all the statistics and also the initial seed number. 

Note: When choosing the path for the fuzzer, we use both a seedable RNG and a weighted index, to be able to reuse a specific seed in order to replay a specific scenario (in case of an unknown and unexpected error).
