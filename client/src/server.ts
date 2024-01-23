export class ServerConnection {
    connection: RTCPeerConnection;
    dataChannel: RTCDataChannel;
    id: number = -1; // god i miss rust but wasm-bindgen sucks with thisss
    iceCollectionId: number;

    constructor() {
        console.log("connecting to server");

        let pc = new RTCPeerConnection();

        this.connection = pc;

        let fetchNumber = 0;

        this.iceCollectionId = setInterval(async () => {
            fetchNumber++;

            if (fetchNumber > 3) {
                clearInterval(this.iceCollectionId);
            }

            let candidates_response = await fetch("/api/ice_candidate");

            let candidates: Array<RTCIceCandidateInit> = (await candidates_response.json());

            candidates.forEach((candidate) => {
                pc.addIceCandidate(candidate);
            });

        }, 1000);

        let dc = pc.createDataChannel("data");

        this.dataChannel = dc;
        this.connect();
    }

    async connect() {
        this.dataChannel.onopen = () => {
            this.dataChannel.send("hello there");
        }

        let sentOffer = false;
        let candidatesToAdd: RTCIceCandidateInit[] = [];

        this.connection.onicecandidate = async (candidate) => {
            if (candidate.candidate == null) return;

            let candidateInit = candidate.candidate.toJSON();

            if (sentOffer) {
                await sendCandidate(candidateInit);
            } else {
                candidatesToAdd.push(candidateInit);
            }
        }

        this.connection.onconnectionstatechange = () => {
            if (this.connection.connectionState == "connected") {
                clearInterval(this.iceCollectionId);
            }

            console.log(`Connection state changed: ${this.connection.connectionState}`);
        };

        let offer = await this.connection.createOffer();
        await this.connection.setLocalDescription(offer);

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

        await this.connection.setRemoteDescription({
            type: "answer",
            sdp: json.sdp,
        });

        this.id = json.id;

        sentOffer = true;

        candidatesToAdd.forEach(async (candidate) => {
            await sendCandidate(candidate);
        });
    }

    sendMessage(message: ArrayBuffer | String) {
        if (typeof(message) == "string") { // my god this is stupid
            this.dataChannel.send(message as string);
        } else {
            this.dataChannel.send(message as ArrayBuffer);
        }
    }
};

async function sendCandidate(candidate: RTCIceCandidateInit) {
    await fetch("/api/ice_candidate", {
        method: "POST",
        body: JSON.stringify(candidate),
        headers: {
            "Content-Type": "application/json",
        },
    });
}
