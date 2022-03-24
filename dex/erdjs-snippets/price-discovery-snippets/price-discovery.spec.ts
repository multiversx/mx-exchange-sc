import { Balance, IProvider, NetworkConfig, ReturnCode, Token, TokenType } from "@elrondnetwork/erdjs";
import { AirdropService, createTokenAmount, ESDTInteractor, ITestSession, ITestUser, TestSession } from "@elrondnetwork/erdjs-snippets";
import BigNumber from "bignumber.js";
import { assert } from "chai";
import { Console } from "console";
import { createInteractor, PriceDiscoveryInitArguments } from "./price-discovery";

namespace StorageKeys {
    export const LaunchedTokenId = "lanchedToken";
    export const ContractAddress = "contractAddress";
}

describe("price discovery snippet", async function () {
    this.bail(true);

    const LotteryName = "fooLottery";

    const LaunchedTokenName = "SoCoolWow";
    const LaunchedTokenTicker = "SOCOOLWOW";
    const LaunchedTokenSupply = "100000000";

    const AcceptedTokenId = "EGLD";
    const ExtraRewardsTokenId = "EGLD";
    const NrBlocksPerPhase = 100; // ~10 minutes
    const UnbondPeriodEpochs = 1;
    const MinPenaltyPercentage = 1_000_000_000_000; // 10%
    const MaxPenaltyPercentage = 5_000_000_000_000; // 50%
    const FixedPenaltyPercentage = 2_500_000_000_000; // 25%

    let suite = this;
    let session: ITestSession;
    let provider: IProvider;
    let whale: ITestUser;
    let owner: ITestUser;

    this.beforeAll(async function () {
        session = await TestSession.loadOnSuite("devnet", suite);
        provider = session.provider;
        whale = session.users.whale;
        owner = session.users.whale;
        await session.syncNetworkConfig();

        console.log("beforeAll called", NetworkConfig.getDefault());
    });

    it("issue lanched token", async function () {
        session.expectLongInteraction(this);

        let interactor = await ESDTInteractor.create(session);
        let token = new Token({
            name: LaunchedTokenName,
            ticker: LaunchedTokenTicker,
            decimals: 0,
            supply: LaunchedTokenSupply,
            type: TokenType.Fungible
        });
        await session.syncUsers([owner]);
        await interactor.issueToken(owner, token);
        await session.saveToken(StorageKeys.LaunchedTokenId, token);
    });

    it("deployPriceDiscovery", async function () {
        await session.syncNetworkConfig();
        console.log("beforeAll called", NetworkConfig.getDefault());

        session.expectLongInteraction(this);

        await session.syncUsers([owner]);

        let interactor = await createInteractor(provider);

        let launchedTokenId = await session.loadToken(StorageKeys.LaunchedTokenId);
        let startBlock = (await provider.getNetworkStatus()).Nonce + 10;
        let deployArgs = new PriceDiscoveryInitArguments(
            launchedTokenId.getTokenIdentifier(),
            AcceptedTokenId,
            ExtraRewardsTokenId,
            new BigNumber("10e+18"), // 1:1 ratio for min price
            startBlock,
            NrBlocksPerPhase,
            NrBlocksPerPhase,
            NrBlocksPerPhase,
            UnbondPeriodEpochs,
            new BigNumber(MinPenaltyPercentage),
            new BigNumber(MaxPenaltyPercentage),
            new BigNumber(FixedPenaltyPercentage)
        );

        let { address, returnCode } = await interactor.deploy(owner, deployArgs);

        assert.isTrue(returnCode.isSuccess());

        await session.saveAddress(StorageKeys.ContractAddress, address);
    });

    it("airdrop EGLD", async function () {
        session.expectLongInteraction(this);

        let amount = Balance.egld(1);
        await session.syncUsers([whale]);
        await AirdropService.createOnSession(session).sendToEachUser(whale, amount);
    });

    it("issue lottery token", async function () {
        session.expectLongInteraction(this);

        let interactor = await ESDTInteractor.create(session);
        let token = new Token({ name: "FOO", ticker: "FOO", decimals: 0, supply: "100000000", type: TokenType.Fungible });
        await session.syncUsers([owner]);
        await interactor.issueToken(owner, token);
        await session.saveToken("lotteryToken", token);
    });

    it("airdrop lottery token", async function () {
        session.expectLongInteraction(this);

        let lotteryToken = await session.loadToken("lotteryToken");
        let amount = createTokenAmount(lotteryToken, "10");
        await session.syncUsers([owner]);
        await AirdropService.createOnSession(session).sendToEachUser(owner, amount);
    });

    it("start lottery", async function () {
        session.expectLongInteraction(this);

        await session.syncUsers([owner]);

        let contractAddress = await session.loadAddress("contractAddress");
        let lotteryToken = await session.loadToken("lotteryToken");
        let interactor = await createInteractor(provider, contractAddress);
        let whitelist = session.users.getAddressesOfFriends();
        let returnCode = await interactor.start(owner, LotteryName, lotteryToken, 1, whitelist);
        assert.isTrue(returnCode.isSuccess());
    });

    it("get lottery info and status", async function () {
        let contractAddress = await session.loadAddress("contractAddress");
        let lotteryToken = await session.loadToken("lotteryToken");
        let interactor = await createInteractor(provider, contractAddress);
        let lotteryInfo = await interactor.getLotteryInfo(LotteryName);
        let lotteryStatus = await interactor.getStatus(LotteryName);
        console.log("Info:", lotteryInfo.valueOf());
        console.log("Prize pool:", lotteryInfo.getFieldValue("prize_pool").toString());
        console.log("Status:", lotteryStatus);

        assert.equal(lotteryInfo.getFieldValue("token_identifier"), lotteryToken.identifier);
        assert.equal(lotteryStatus, "Running");
    });

    it("get whitelist", async function () {
        let contractAddress = await session.loadAddress("contractAddress");
        let interactor = await createInteractor(provider, contractAddress);
        let whitelist = await interactor.getWhitelist(LotteryName);
        console.log("Whitelist:", whitelist);

        assert.deepEqual(whitelist, session.users.getAddressesOfFriends());
    });

    it("friends buy tickets", async function () {
        session.expectLongInteraction(this);

        await session.syncAllUsers();

        let contractAddress = await session.loadAddress("contractAddress");
        let lotteryToken = await session.loadToken("lotteryToken");
        let interactor = await createInteractor(provider, contractAddress);

        let buyAmount = createTokenAmount(lotteryToken, "1");
        let buyPromises = session.users.getFriends().map(friend => interactor.buyTicket(friend, LotteryName, buyAmount));
        let returnCodes: ReturnCode[] = await Promise.all(buyPromises);

        for (const returnCode of returnCodes) {
            assert.isTrue(returnCode.isSuccess());
        }
    });
});
