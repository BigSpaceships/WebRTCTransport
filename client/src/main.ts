import './style.css'

async function createConnection() {
    console.log("creating connection");

    let pc = new RTCPeerConnection();

    pc.createDataChannel("data");

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

document.getElementById("app")?.addEventListener("click", async () => {
    createConnection();
});
