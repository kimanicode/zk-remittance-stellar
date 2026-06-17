use ark_bls12_381::{Fq, Fq2, G1Affine, G2Affine};
use ark_serialize::CanonicalSerialize;
use num_bigint::BigUint;
use serde::Deserialize;
use std::{fs, str::FromStr};

#[derive(Deserialize)]
struct VerificationKeyJson {
    vk_alpha_1: [String; 3],
    vk_beta_2: [[String; 2]; 3],
    vk_gamma_2: [[String; 2]; 3],
    vk_delta_2: [[String; 2]; 3],
    #[serde(rename = "IC")]
    ic: Vec<[String; 3]>,
}

#[derive(Deserialize)]
struct ProofJson {
    pi_a: [String; 3],
    pi_b: [[String; 2]; 3],
    pi_c: [String; 3],
}

type PublicSignalsJson = Vec<String>;

fn g1_bytes(x: &str, y: &str) -> Vec<u8> {
    let p = G1Affine::new(
        Fq::from_str(x).expect("invalid G1 x"),
        Fq::from_str(y).expect("invalid G1 y"),
    );
    let mut out = Vec::new();
    p.serialize_uncompressed(&mut out).expect("failed to serialize G1");
    out
}

fn g2_bytes(x1: &str, x2: &str, y1: &str, y2: &str) -> Vec<u8> {
    let x = Fq2::new(Fq::from_str(x1).expect("invalid G2 x1"), Fq::from_str(x2).expect("invalid G2 x2"));
    let y = Fq2::new(Fq::from_str(y1).expect("invalid G2 y1"), Fq::from_str(y2).expect("invalid G2 y2"));
    let p = G2Affine::new(x, y);
    let mut out = Vec::new();
    p.serialize_uncompressed(&mut out).expect("failed to serialize G2");
    out
}

fn parse_u256_be(signal: &str) -> [u8; 32] {
    let n = BigUint::parse_bytes(signal.as_bytes(), 10).expect("invalid public signal");
    let mut raw = n.to_bytes_be();
    if raw.len() > 32 {
        panic!("public signal exceeds 256 bits");
    }
    if raw.len() < 32 {
        let mut padded = vec![0u8; 32 - raw.len()];
        padded.append(&mut raw);
        raw = padded;
    }
    raw.try_into().expect("invalid 32-byte conversion")
}

fn vk_hex(path: &str) -> String {
    let src = fs::read_to_string(path).expect("failed to read vk json");
    let vk: VerificationKeyJson = serde_json::from_str(&src).expect("invalid vk json");

    eprintln!("alpha_1: {:?}", vk.vk_alpha_1);
    eprintln!("beta_2: {:?}", vk.vk_beta_2);
    eprintln!("gamma_2: {:?}", vk.vk_gamma_2);
    eprintln!("delta_2: {:?}", vk.vk_delta_2);
    eprintln!("IC count: {}", vk.ic.len());
    for (i, p) in vk.ic.iter().enumerate() {
        eprintln!("IC[{}]: {:?}", i, p);
    }

    let mut out = Vec::new();
    eprintln!("encoding alpha...");
    out.extend(g1_bytes(&vk.vk_alpha_1[0], &vk.vk_alpha_1[1]));
    eprintln!("encoding beta...");
    out.extend(g2_bytes(&vk.vk_beta_2[0][0], &vk.vk_beta_2[0][1], &vk.vk_beta_2[1][0], &vk.vk_beta_2[1][1]));
    eprintln!("encoding gamma...");
    out.extend(g2_bytes(&vk.vk_gamma_2[0][0], &vk.vk_gamma_2[0][1], &vk.vk_gamma_2[1][0], &vk.vk_gamma_2[1][1]));
    eprintln!("encoding delta...");
    out.extend(g2_bytes(&vk.vk_delta_2[0][0], &vk.vk_delta_2[0][1], &vk.vk_delta_2[1][0], &vk.vk_delta_2[1][1]));

    out.extend((vk.ic.len() as u32).to_be_bytes());
    for (i, point) in vk.ic.iter().enumerate() {
        eprintln!("encoding IC[{}]...", i);
        out.extend(g1_bytes(&point[0], &point[1]));
    }

    hex::encode(out)
}

fn proof_hex(path: &str) -> String {
    let src = fs::read_to_string(path).expect("failed to read proof json");
    let proof: ProofJson = serde_json::from_str(&src).expect("invalid proof json");

    let mut out = Vec::new();
    out.extend(g1_bytes(&proof.pi_a[0], &proof.pi_a[1]));
    out.extend(g2_bytes(&proof.pi_b[0][0], &proof.pi_b[0][1], &proof.pi_b[1][0], &proof.pi_b[1][1]));
    out.extend(g1_bytes(&proof.pi_c[0], &proof.pi_c[1]));

    hex::encode(out)
}

fn public_hex(path: &str) -> String {
    let src = fs::read_to_string(path).expect("failed to read public json");
    let signals: PublicSignalsJson = serde_json::from_str(&src).expect("invalid public json");

    let mut out = Vec::new();
    out.extend((signals.len() as u32).to_be_bytes());
    for s in signals {
        out.extend(parse_u256_be(&s));
    }

    hex::encode(out)
}

fn main() {
    let mut args = std::env::args().skip(1);
    let kind = args.next().expect("usage: vk-encoder <vk|proof|public> <json-file>");
    let path = args.next().expect("usage: vk-encoder <vk|proof|public> <json-file>");

    let hex = match kind.as_str() {
        "vk" => vk_hex(&path),
        "proof" => proof_hex(&path),
        "public" => public_hex(&path),
        _ => panic!("unknown kind: {kind}"),
    };

    println!("{hex}");
}

#[cfg(test)]
mod debug_tests {
    use super::*;

    #[test]
    fn test_alpha() {
        g1_bytes(
            "2649501615851480174493885748867565568866510194441427851661955018858410241885439010956020730186857309596148236633043",
            "2865123248692925047969414699021911752480979023824929130302951135036102381002360508846346932801352924172463744014724",
        );
    }
}

    #[test]
    fn test_beta() {
        g2_bytes(
            "2414104591888876650069604362929597095847920676900652154923817230952404680432288514939633348818190605180490772856717",
            "1199604760900938375893945666544983469253472962158764853043270448061833340031132502604823643622066627011225280420606",
            "1726966200769802608694965510187839204090704093141967221882516406936534999951768955304084795115163394309101727003922",
            "1834804291247630985929573076036514142538810055182041488756495340648922072359922183693973019362202435627030928541719",
        );
    }

    #[test]
    fn test_gamma() {
        g2_bytes(
            "352701069587466618187139116011060144890029952792775240219908644239793785735715026873347600343865175952761926303160",
            "3059144344244213709971259814753781636986470325476647558659373206291635324768958432433509563104347017837885763365758",
            "1985150602287291935568054521177171638300868978215655730859378665066344726373823718423869104263333984641494340347905",
            "927553665492332455747201965776037880757740193453592970025027978793976877002675564980949289727957565575433344219582",
        );
    }

    #[test]
    fn test_delta() {
        g2_bytes(
            "1475184967471019307324932376074667085623712920248914514489926851309250342243672347222784477411924893735878290325944",
            "3480826791071114618057181143850592191370548467562106248350569881197335288606737267604024093217050460774102348115862",
            "3236700669987469909010443561940107010031994342335708084249072812307873451300530973401597139318010829070510175424900",
            "1883442821974671675996915135196253573041044974214457578709377936188804606835105527749690108596758409165312369593165",
        );
    }

    #[test]
    fn test_ic0() {
        g1_bytes(
            "1216013473287580154263783332529718136066868805166632594977530709183362773123246176718705477481220163074022221719479",
            "2272680314855873469760478090370300862844080003375035295025083443696751966762909832845963531454797841822335005193598",
        );
    }

    #[test]
    fn test_ic1() {
        g1_bytes(
            "592329699744677217622163403971841425094805971844243706136068317339253579773765182341621825472934478960434613545198",
            "1764790449412636502586960945811035915024787074442277616906111808225401315419951963082593095898935696892793438428740",
        );
    }

    #[test]
    fn test_ic2() {
        g1_bytes(
            "2713110078587707691844187760136194239913283483896664788840238794154203088193009436933112665998404069410691015237437",
            "193948047057789004262501747359508869386619125758649415821014553027015469661864856818960561864422089734045683435265",
        );
    }

    #[test]
    fn test_ic3() {
        g1_bytes(
            "3978260853992828902112998090565385086046844361432882747294034808039317042977205718875183328158324324281427681115432",
            "179703188318558869742566852409972934207091774557490084216716911881917669086882091225970482695149572202769267652606",
        );
    }
