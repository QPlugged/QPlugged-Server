import { spawn } from "child_process";
import { resourceDir } from "./env";
import { tmpdir } from "os";
import { randomUUID } from "crypto";
import path from "path";
import { readFile } from "fs/promises";

export async function silkToPcm(
    from: string,
    sampleRate: number,
): Promise<Buffer> {
    const tmpfile = path.join(tmpdir(), randomUUID());
    await new Promise<void>((resolve) =>
        spawn(
            path.join(
                resourceDir,
                `silk-codec${process.platform === "win32" ? ".exe" : ""}`,
            ),
            ["stp", "-i", from, "-o", tmpfile, "-s", sampleRate.toString()],
            { stdio: "inherit", windowsHide: true },
        ).on("exit", () => resolve()),
    );
    return await readFile(tmpfile);
}

export function pcmToWav(
    pcm: Uint8Array,
    channels: number,
    sampleRate: number,
    bitsPerSample: number,
) {
    const wavHeader = {
        // OFFS SIZE NOTES
        // 0    4    "RIFF" = 0x52494646
        chunkId: [0x52, 0x49, 0x46, 0x46],
        // 4    4    36+SubChunk2Size = 4+(8+SubChunk1Size)+(8+SubChunk2Size)
        chunkSize: 0,
        // 8    4    "WAVE" = 0x57415645
        format: [0x57, 0x41, 0x56, 0x45],
        // 12   4    "fmt " = 0x666d7420
        subChunk1Id: [0x66, 0x6d, 0x74, 0x20],
        // 16   4    16 for PCM
        subChunk1Size: 16,
        // 20   2    PCM = 1
        audioFormat: 1,
        // 22   2    Mono = 1, Stereo = 2...
        numChannels: channels,
        // 24   4    8000, 44100...
        sampleRate: sampleRate,
        // 28   4    SampleRate*NumChannels*BitsPerSample/8
        byteRate: 0,
        // 32   2    NumChannels*BitsPerSample/8
        blockAlign: 0,
        // 34   2    8 bits = 8, 16 bits = 16
        bitsPerSample: bitsPerSample,
        // 36   4    "data" = 0x64617461
        subChunk2Id: [0x64, 0x61, 0x74, 0x61],
        // 40   4    data size = NumSamples*NumChannels*BitsPerSample/8
        subChunk2Size: 0,
    };
    function u32ToArray(i: number) {
        return [i & 0xff, (i >> 8) & 0xff, (i >> 16) & 0xff, (i >> 24) & 0xff];
    }
    function u16ToArray(i: number) {
        return [i & 0xff, (i >> 8) & 0xff];
    }

    wavHeader.blockAlign =
        (wavHeader.numChannels * wavHeader.bitsPerSample) >> 3;
    wavHeader.byteRate = wavHeader.blockAlign * wavHeader.sampleRate;
    wavHeader.subChunk2Size = pcm.length * (wavHeader.bitsPerSample >> 3);
    wavHeader.chunkSize = 36 + wavHeader.subChunk2Size;

    const wavHeaderData = new Uint8Array(
        wavHeader.chunkId.concat(
            u32ToArray(wavHeader.chunkSize),
            wavHeader.format,
            wavHeader.subChunk1Id,
            u32ToArray(wavHeader.subChunk1Size),
            u16ToArray(wavHeader.audioFormat),
            u16ToArray(wavHeader.numChannels),
            u32ToArray(wavHeader.sampleRate),
            u32ToArray(wavHeader.byteRate),
            u16ToArray(wavHeader.blockAlign),
            u16ToArray(wavHeader.bitsPerSample),
            wavHeader.subChunk2Id,
            u32ToArray(wavHeader.subChunk2Size),
        ),
    );
    const wavData = new Uint8Array(wavHeaderData.length + pcm.length);
    wavData.set(wavHeaderData);
    wavData.set(pcm, wavHeaderData.length);

    return wavData;
}
