use alloy::sol;

sol! {
    #[derive(Debug)]
    #[sol(rpc)]
    interface IVeaInboxArbToEth {
        event MessageSent(bytes _nodeData);
        event SnapshotSaved(bytes32 _snapshot, uint256 _epoch, uint64 _count);
        event SnapshotSent(uint256 indexed _epochSent, bytes32 _ticketId);
    }

    #[derive(Debug)]
    #[sol(rpc)]
    interface IVeaOutboxArbToEth {
        struct Claim {
            bytes32 stateRoot;
            address claimer;
            uint32 timestampClaimed;
            uint32 timestampVerification;
            uint32 blocknumberVerification;
            Party honest;
            address challenger;
        }

        enum Party {
            None,
            Claimer,
            Challenger
        }

        event Claimed(address indexed _claimer, uint256 indexed _epoch, bytes32 indexed _stateRoot);
        event Challenged(uint256 indexed _epoch, address indexed _challenger);
        event MessageRelayed(uint64 _msgId);
        event VerificationStarted(uint256 indexed _epoch);
        event Verified(uint256 indexed _epoch);
    }
}