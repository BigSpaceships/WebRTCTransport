import './style.css'

async function createConnection() {
    console.log("creating connection");

    let pc = new RTCPeerConnection();

    let fetchNumber = 0;

    let iceCollectionId = setInterval(async () => {
        fetchNumber++;

        if (fetchNumber > 3) {
            clearInterval(iceCollectionId);
        }

        let candidates_response = await fetch("/api/ice_candidate");

        let candidates: Array<RTCIceCandidateInit> = (await candidates_response.json());

        candidates.forEach((candidate) => {
            pc.addIceCandidate(candidate);
        });

    }, 1000);
    
    let dc = pc.createDataChannel("data");

    pc.onicecandidate = async (candidate) => {
        if (candidate.candidate == null) return;

        let response = await fetch("/api/ice_candidate", {
            method: "POST",
            body: JSON.stringify(candidate.candidate.toJSON()),
            headers: {
                "Content-Type": "application/json",
            },
        });
    }

    pc.onconnectionstatechange = () => {
        if (pc.connectionState == "connected") {
            clearInterval(iceCollectionId);
        }

        console.log(`Connection state changed: ${pc.connectionState}`);
    };

    let offer = await pc.createOffer();
    await pc.setLocalDescription(offer);

    let response = await fetch("/api/new_offer", {
        method: "POST",
        body: JSON.stringify({
            sdp: offer.sdp
        }),
        headers: {
            "Content-Type": "application/json",
        },
    });

    let json = await response.json();

    await pc.setRemoteDescription({
        type: "answer",
        sdp: json.sdp,
    });
}

document.getElementById("connect")?.addEventListener("click", async () => {
    createConnection();
});
