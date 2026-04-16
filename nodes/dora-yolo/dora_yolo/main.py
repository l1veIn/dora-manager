"""Object detection node using Ultralytics YOLOv8.

Accepts image frames and outputs bounding boxes with confidence scores,
class labels, and annotated images with detections drawn.
"""

import argparse
import os

import cv2
import numpy as np
import pyarrow as pa
from dora import Node
from ultralytics import YOLO


def main():
    """Run YOLO object detection node."""
    parser = argparse.ArgumentParser(
        description="Ultralytics YOLO: Object detection using YOLOv8 models.",
    )

    parser.add_argument(
        "--name",
        type=str,
        required=False,
        help="The name of the node in the dataflow.",
        default="ultralytics-yolo",
    )
    parser.add_argument(
        "--model",
        type=str,
        required=False,
        help="The name of the model file (e.g. yolov8n.pt).",
        default="yolov8n.pt",
    )

    args = parser.parse_args()

    model_path = os.getenv("MODEL", args.model)
    bbox_format = os.getenv("FORMAT", "xyxy")
    confidence = float(os.getenv("CONFIDENCE", "0.25"))

    print(f"[dora-yolo] Loading model: {model_path}")
    model = YOLO(model_path)
    print(f"[dora-yolo] Model loaded. conf={confidence}, format={bbox_format}")

    node = Node(args.name)

    pa.array([])  # initialize pyarrow array

    for event in node:
        event_type = event["type"]

        if event_type == "INPUT":
            event_id = event["id"]

            if event_id == "confidence":
                # Runtime confidence override from dm-slider or similar
                val = event["value"].to_numpy()
                if len(val) > 0:
                    confidence = float(val[0])
                    confidence = max(0.0, min(1.0, confidence))
                    print(f"[dora-yolo] Confidence updated: {confidence:.2f}")

            elif event_id == "image":
                storage = event["value"]
                metadata = event["metadata"]
                encoding = metadata["encoding"]
                width = metadata["width"]
                height = metadata["height"]

                if encoding in ("bgr8", "rgb8"):
                    channels = 3
                    storage_type = np.uint8
                elif encoding in ("jpeg", "jpg", "png"):
                    raw = storage.to_numpy()
                    frame = cv2.imdecode(raw, cv2.IMREAD_COLOR)
                    if frame is None:
                        print("[dora-yolo] Failed to decode image")
                        continue
                    encoding = "bgr8"
                    height, width = frame.shape[:2]
                    channels = 3
                    storage_type = np.uint8
                    # Skip the generic reshape below
                    frame = frame[:, :, ::-1]  # BGR to RGB
                else:
                    raise RuntimeError(f"Unsupported image encoding: {encoding}")

                if encoding != "jpeg" and encoding != "jpg" and encoding != "png":
                    frame = (
                        storage.to_numpy()
                        .astype(storage_type)
                        .reshape((height, width, channels))
                    )
                    if encoding == "bgr8":
                        frame = frame[:, :, ::-1]  # BGR to RGB

                results = model(frame, verbose=False, conf=confidence)

                # --- Output 1: bbox ---
                if bbox_format == "xyxy":
                    bboxes = np.array(results[0].boxes.xyxy.cpu())
                elif bbox_format == "xywh":
                    bboxes = np.array(results[0].boxes.xywh.cpu())
                else:
                    raise RuntimeError(f"Unsupported bbox format: {bbox_format}")

                conf = np.array(results[0].boxes.conf.cpu())
                labels = np.array(results[0].boxes.cls.cpu())
                names = [model.names.get(int(label)) for label in labels]

                num_detections = len(bboxes)
                bbox_data = {
                    "bbox": bboxes.ravel(),
                    "conf": conf,
                    "labels": names,
                }
                bbox_array = pa.array([bbox_data])

                out_metadata = dict(metadata) if metadata else {}
                out_metadata["format"] = bbox_format
                out_metadata["primitive"] = "boxes2d"

                node.send_output("bbox", bbox_array, out_metadata)

                # --- Output 2: annotated_image ---
                annotated = results[0].plot()  # BGR numpy array with boxes drawn
                ret, jpeg = cv2.imencode(".jpg", annotated)
                if ret:
                    node.send_output(
                        "annotated_image",
                        pa.array(jpeg.ravel()),
                        {
                            "encoding": "jpeg",
                            "width": annotated.shape[1],
                            "height": annotated.shape[0],
                            "num_detections": num_detections,
                        },
                    )

        elif event_type == "ERROR":
            print(f"[dora-yolo] Received dora error: {event['error']}")


if __name__ == "__main__":
    main()
