import os
import json
from typing import Any
from cog import BasePredictor, Input, Path
import torch
from PIL import Image
from torchvision import transforms


class Predictor(BasePredictor):
    def setup(self):
        """Load the model into memory to make running multiple predictions efficient"""
        self.model = torch.hub.load(
            "pytorch/vision:v0.10.0", "resnet18", pretrained=True
        )
        self.model.eval()

        self.preprocess = transforms.Compose(
            [
                transforms.Resize(256),
                transforms.CenterCrop(224),
                transforms.ToTensor(),
                transforms.Normalize(
                    mean=[0.485, 0.456, 0.406], std=[0.229, 0.224, 0.225]
                ),
            ]
        )

        # Get the directory of the current script
        current_dir = os.path.dirname(__file__)
        # Construct the path to imagenet-simple-labels.json in the same directory
        json_file = os.path.join(current_dir, "imagenet-simple-labels.json")

        # Load JSON file with labels
        with open(json_file, "r") as f:
            self.categories = json.load(f)

    def predict(self, image: Path = Input(description="Image to classify")) -> Any:
        """Run a single prediction on the model"""
        # Preprocess the image
        input_image = Image.open(image)

        # Convert the image to RGB if it has an alpha channel (RGBA)
        if input_image.mode == 'RGBA':
            input_image = input_image.convert('RGB')

        input_tensor = self.preprocess(input_image)
        # create a mini-batch as expected by the model
        input_batch = input_tensor.unsqueeze(0)
        with torch.no_grad():
            output = self.model(input_batch)
        # Return the top 5 predictions
        probabilities = torch.nn.functional.softmax(output[0], dim=0)
        top5_prob, top5_catid = torch.topk(probabilities, 5)
        res_list = []
        for i in range(top5_prob.size(0)):
            res_list.append([self.categories[top5_catid[i]], top5_prob[i].item()])
        return res_list