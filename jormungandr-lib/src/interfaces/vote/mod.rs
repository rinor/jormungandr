use crate::{
    crypto::hash::Hash,
    interfaces::{
        account_identifier::AccountIdentifier, blockdate::BlockDateDef, stake::Stake,
        value::ValueDef,
    },
};
use chain_impl_mockchain::{
    certificate::{ExternalProposalId, Proposal, Proposals, VoteAction, VotePlan},
    header::BlockDate,
    ledger::governance::{ParametersGovernanceAction, TreasuryGovernanceAction},
    value::Value,
    vote::{self, Options, PayloadType},
};
use chain_vote::MemberPublicKey;
use core::ops::Range;
use serde::de::{SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
use std::convert::TryInto;
use typed_bytes::ByteBuilder;

#[derive(
    Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, serde::Deserialize,
)]
#[serde(remote = "PayloadType", rename_all = "snake_case")]
enum PayloadTypeDef {
    Public,
    Private,
}

struct SerdeMemberPublicKey(chain_vote::MemberPublicKey);

impl<'de> Deserialize<'de> for SerdeMemberPublicKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        struct PublicKeyVisitor;
        impl<'de> Visitor<'de> for PublicKeyVisitor {
            type Value = SerdeMemberPublicKey;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter
                    .write_str("Expected a compatible hex representation of required public key")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                self.visit_string(value.to_string())
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let content = hex::decode(&v).map_err(|err| {
                    serde::de::Error::custom(format!(
                        "Invalid public key hex representation {}, with err: {}",
                        &v, err
                    ))
                })?;
                Ok(SerdeMemberPublicKey(
                    MemberPublicKey::from_bytes(&content).ok_or_else(|| {
                        serde::de::Error::custom(format!(
                            "Invalid public key with hex representation {}",
                            &v
                        ))
                    })?,
                ))
            }
        }
        deserializer.deserialize_string(PublicKeyVisitor)
    }
}

impl Serialize for SerdeMemberPublicKey {
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&hex::encode(&self.0.to_bytes()))
    }
}

#[derive(Deserialize)]
#[serde(remote = "VotePlan")]
pub struct VotePlanDef {
    #[serde(with = "PayloadTypeDef", getter = "payload_type")]
    payload_type: PayloadType,
    #[serde(with = "BlockDateDef", getter = "vote_start")]
    vote_start: BlockDate,
    #[serde(with = "BlockDateDef", getter = "vote_end")]
    vote_end: BlockDate,
    #[serde(with = "BlockDateDef", getter = "committee_end")]
    committee_end: BlockDate,
    #[serde(deserialize_with = "deserialize_proposals", getter = "proposals")]
    proposals: Proposals,
    #[serde(
        deserialize_with = "serde_committee_member_public_keys::deserialize",
        getter = "committee_member_public_keys",
        default = "Vec::new"
    )]
    committee_member_public_keys: Vec<chain_vote::MemberPublicKey>,
}

#[derive(Deserialize)]
#[serde(remote = "Proposal")]
struct VoteProposalDef {
    #[serde(
        deserialize_with = "deserialize_external_proposal_id",
        getter = "external_id"
    )]
    external_id: ExternalProposalId,
    #[serde(deserialize_with = "deserialize_choices", getter = "options")]
    options: Options,
    #[serde(with = "VoteActionDef", getter = "action")]
    action: VoteAction,
}

#[derive(Deserialize)]
#[serde(remote = "VoteAction", rename_all = "snake_case")]
enum VoteActionDef {
    OffChain,
    #[serde(with = "TreasuryGovernanceActionDef")]
    Treasury {
        action: TreasuryGovernanceAction,
    },
    #[serde(with = "ParametersGovernanceActionDef")]
    Parameters {
        action: ParametersGovernanceAction,
    },
}

#[derive(Deserialize)]
#[serde(remote = "ParametersGovernanceAction", rename_all = "snake_case")]
enum ParametersGovernanceActionDef {
    RewardAdd {
        #[serde(with = "ValueDef")]
        value: Value,
    },
}

#[derive(Deserialize)]
#[serde(remote = "TreasuryGovernanceAction", rename_all = "snake_case")]
enum TreasuryGovernanceActionDef {
    TransferToRewards {
        #[serde(with = "ValueDef")]
        value: Value,
    },
}

impl From<VotePlanDef> for VotePlan {
    fn from(vpd: VotePlanDef) -> Self {
        Self::new(
            vpd.vote_start,
            vpd.vote_end,
            vpd.committee_end,
            vpd.proposals,
            vpd.payload_type,
            vpd.committee_member_public_keys,
        )
    }
}

mod serde_committee_member_public_keys {
    use crate::interfaces::vote::SerdeMemberPublicKey;
    use serde::de::{SeqAccess, Visitor};
    use serde::ser::SerializeSeq;
    use serde::{Deserializer, Serialize, Serializer};

    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<Vec<chain_vote::MemberPublicKey>, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct PublicKeysSeqVisitor;
        impl<'de> Visitor<'de> for PublicKeysSeqVisitor {
            type Value = Vec<SerdeMemberPublicKey>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("Invalid sequence of hex encoded public keys")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, <A as SeqAccess<'de>>::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut result = Vec::with_capacity(seq.size_hint().unwrap_or(0));
                while let Some(key) = seq.next_element()? {
                    result.push(key);
                }
                Ok(result)
            }
        }
        let keys = deserializer.deserialize_seq(PublicKeysSeqVisitor {})?;
        Ok(keys.iter().map(|key| key.0.clone()).collect())
    }

    pub fn serialize<S>(
        keys: &Vec<chain_vote::MemberPublicKey>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(keys.len()))?;
        for key in keys {
            seq.serialize_element(&SerdeMemberPublicKey(key.clone()));
        }
        seq.end()
    }
}

impl From<VoteProposalDef> for Proposal {
    fn from(vpd: VoteProposalDef) -> Self {
        Self::new(vpd.external_id, vpd.options, vpd.action)
    }
}

fn deserialize_external_proposal_id<'de, D>(deserializer: D) -> Result<ExternalProposalId, D::Error>
where
    D: Deserializer<'de>,
{
    struct ExternalProposalIdVisitor;

    impl<'de> serde::de::Visitor<'de> for ExternalProposalIdVisitor {
        type Value = ExternalProposalId;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("external proposal id in a hexadecimal form")
        }

        fn visit_str<E>(self, value: &str) -> Result<ExternalProposalId, E>
        where
            E: serde::de::Error,
        {
            Ok(std::str::FromStr::from_str(value).unwrap())
        }
    }

    deserializer.deserialize_str(ExternalProposalIdVisitor)
}

fn deserialize_choices<'de, D>(deserializer: D) -> Result<Options, D::Error>
where
    D: Deserializer<'de>,
{
    struct OptionsVisitor;

    impl<'de> serde::de::Visitor<'de> for OptionsVisitor {
        type Value = Options;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("options number must be an integer less than 256")
        }

        fn visit_u64<E>(self, value: u64) -> Result<Options, E>
        where
            E: serde::de::Error,
        {
            if value > 255 {
                return Err(serde::de::Error::custom("expecting a value less than 256"));
            }
            Options::new_length(value as u8).map_err(|err| serde::de::Error::custom(err))
        }
    }

    deserializer.deserialize_u64(OptionsVisitor)
}

fn deserialize_proposals<'de, D>(deserializer: D) -> Result<Proposals, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    struct ProposalInternal(#[serde(with = "VoteProposalDef")] Proposal);

    #[derive(Deserialize)]
    struct ProposalsList(Vec<ProposalInternal>);

    let proposals_list = ProposalsList::deserialize(deserializer)?;
    let mut proposals = Proposals::new();
    for proposal in proposals_list.0.into_iter() {
        match proposals.push(proposal.0) {
            chain_impl_mockchain::certificate::PushProposal::Full { .. } => {
                panic!("too much proposals")
            }
            _ => {}
        }
    }
    Ok(proposals)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VotePlanStatus {
    pub id: Hash,
    #[serde(with = "PayloadTypeDef")]
    pub payload: PayloadType,
    #[serde(with = "BlockDateDef")]
    pub vote_start: BlockDate,
    #[serde(with = "BlockDateDef")]
    pub vote_end: BlockDate,
    #[serde(with = "BlockDateDef")]
    pub committee_end: BlockDate,
    #[serde(with = "serde_committee_member_public_keys")]
    pub committee_member_keys: Vec<MemberPublicKey>,
    pub proposals: Vec<VoteProposalStatus>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Tally {
    Public { result: TallyResult },
    Private { state: PrivateTallyState },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct TallyResult {
    results: Vec<u64>,
    options: Range<u8>,
}

impl TallyResult {
    pub fn results(&self) -> Vec<u64> {
        self.results.clone()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncryptedTally(Vec<u8>);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrivateTallyState {
    Encrypted {
        encrypted_tally: EncryptedTally,
        total_stake: Stake,
    },
    Decrypted {
        result: TallyResult,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Payload {
    Public {
        choice: u8,
    },
    // We serialize deserialize the bytes directly
    Private {
        encrypted_vote: Vec<u8>,
        proof: Vec<u8>,
    },
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VoteProposalStatus {
    pub index: u8,
    pub proposal_id: Hash,
    pub options: Range<u8>,
    pub tally: Option<Tally>,
    pub votes: HashMap<AccountIdentifier, Payload>,
}

impl From<vote::Payload> for Payload {
    fn from(this: vote::Payload) -> Self {
        match this {
            vote::Payload::Public { choice } => Self::Public {
                choice: choice.as_byte(),
            },
            vote::Payload::Private {
                encrypted_vote,
                proof,
            } => Self::Private {
                encrypted_vote: encrypted_vote.iter().flat_map(|ct| ct.to_bytes()).collect(),
                // TODO: Use iter instead of bytes here
                proof: ByteBuilder::<u8>::new()
                    .u64(proof.len() as u64)
                    .bytes(
                        &proof
                            .ibas()
                            .flat_map(|iba| iba.to_bytes())
                            .collect::<Vec<u8>>(),
                    )
                    .bytes(&proof.ds().flat_map(|ds| ds.to_bytes()).collect::<Vec<u8>>())
                    .bytes(
                        &proof
                            .zwvs()
                            .flat_map(|zwvs| zwvs.to_bytes())
                            .collect::<Vec<u8>>(),
                    )
                    .bytes(&proof.r().to_bytes())
                    .finalize_as_vec(),
            },
        }
    }
}

impl Payload {
    pub fn choice(&self) -> Option<u8> {
        match self {
            Payload::Public { choice } => Some(*choice),
            Payload::Private { .. } => None,
        }
    }
}

impl From<vote::TallyResult> for TallyResult {
    fn from(this: vote::TallyResult) -> Self {
        Self {
            results: this.results().iter().map(|v| (*v).into()).collect(),
            options: this.options().choice_range().clone(),
        }
    }
}

impl From<chain_vote::TallyResult> for TallyResult {
    fn from(this: chain_vote::TallyResult) -> Self {
        Self {
            results: this.votes.iter().map(|w| w.unwrap_or(0).into()).collect(),
            options: (0..this.votes.len().try_into().unwrap()),
        }
    }
}

impl From<vote::Tally> for Tally {
    fn from(this: vote::Tally) -> Self {
        match this {
            vote::Tally::Public { result } => Tally::Public {
                result: result.into(),
            },
            vote::Tally::Private { state } => Tally::Private {
                state: match state {
                    vote::PrivateTallyState::Encrypted {
                        encrypted_tally,
                        total_stake,
                    } => PrivateTallyState::Encrypted {
                        encrypted_tally: EncryptedTally(encrypted_tally.to_bytes()),
                        total_stake: total_stake.into(),
                    },
                    vote::PrivateTallyState::Decrypted { result } => PrivateTallyState::Decrypted {
                        result: result.into(),
                    },
                },
            },
        }
    }
}

impl From<vote::VoteProposalStatus> for VoteProposalStatus {
    fn from(this: vote::VoteProposalStatus) -> Self {
        Self {
            index: this.index,
            proposal_id: this.proposal_id.into(),
            options: this.options.choice_range().clone(),
            tally: this.tally.map(|t| t.into()),
            votes: this
                .votes
                .iter()
                .map(|(k, p)| (k.clone().into(), p.clone().into()))
                .collect(),
        }
    }
}

impl From<vote::VotePlanStatus> for VotePlanStatus {
    fn from(this: vote::VotePlanStatus) -> Self {
        Self {
            id: this.id.into(),
            vote_start: this.vote_start,
            vote_end: this.vote_end,
            committee_end: this.committee_end,
            payload: this.payload,
            committee_member_keys: this.committee_public_keys,
            proposals: this.proposals.into_iter().map(|p| p.into()).collect(),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::interfaces::vote::{serde_committee_member_public_keys, SerdeMemberPublicKey};
    use rand_chacha::rand_core::SeedableRng;

    #[test]
    fn test_deserialize_member_public_keys() {
        let mut rng = rand_chacha::ChaChaRng::from_entropy();
        let crs = chain_vote::CRS::random(&mut rng);
        let comm_key = chain_vote::MemberCommunicationKey::new(&mut rng);

        let member_key =
            chain_vote::MemberState::new(&mut rng, 1, &crs, &[comm_key.to_public()], 0);
        let pk = member_key.public_key();
        let pks = vec![hex::encode(pk.to_bytes())];
        let json = serde_json::to_string(&pks).unwrap();

        let result: Vec<SerdeMemberPublicKey> = serde_json::from_str(&json).unwrap();
        assert_eq!(result[0].0, pk);

        let mut json_deserializer = serde_json::Deserializer::from_str(&json);
        let result =
            serde_committee_member_public_keys::deserialize(&mut json_deserializer).unwrap();
        assert_eq!(result[0], pk);
    }
}
