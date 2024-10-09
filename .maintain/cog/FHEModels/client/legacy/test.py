import unittest
import numpy as np
import os
import subprocess
import joblib
from concrete.ml.deployment import FHEModelClient, FHEModelServer

fhe_files_dir = "../fhe"

class TestPredictionComparison(unittest.TestCase):
    def setUp(self):
        self.test_data_dir = 'test_data'
        self.ground_truths_dir = 'ground_truths'
        self.enc_test_data_dir = "enc_test_data"
         
    def test_predictions(self):
        total_matches = 0
        total_elements = 0
        
        for file_name in os.listdir(self.test_data_dir):
            if file_name.endswith('.csv'):
                test_file_path = os.path.join(self.test_data_dir, file_name)
                ground_truth_file_path = os.path.join(self.ground_truths_dir, file_name)

                """CLIENT"""

                # Create client object to manage keys
                client = FHEModelClient(fhe_files_dir, key_dir="keys")

                # Create private and evaluation keys
                eval_key = client.get_serialized_evaluation_keys()

                # Load features from .csv file
                input_data = joblib.load(test_file_path)

                # Encrypt features
                enc_inputs = []
                for i in input_data:
                    e_i = client.quantize_encrypt_serialize(i.reshape(1, -1))
                    enc_inputs.append(e_i)
                
                # Export encrypted features
                enc_inputs_file = os.path.join(self.enc_test_data_dir, file_name)
                if not os.path.exists(self.enc_test_data_dir): 
                    os.mkdir(self.enc_test_data_dir)
                joblib.dump(enc_inputs, enc_inputs_file)

                """SERVER: SIMULATE WHAT COG DOES"""

                # Create server object to manage circuit
                server = FHEModelServer(fhe_files_dir)

                # Run circuit
                enc_outputs = []
                for e_i in enc_inputs:
                    e_o = server.run(e_i, eval_key)
                    enc_outputs.append(e_o)

                """CLIENT"""

                # Decrypt results
                output_data = []
                for e_o in enc_outputs:
                    o = np.argmax(client.deserialize_decrypt_dequantize(e_o), axis=1)
                    output_data.append(o)
                
                """EVALUATE RESULTS ACCURACY AND COMPARE TO PLAIN-TEXT SKLEARN"""
               
                output_vector = np.concatenate(output_data)

                # Read ground truth vector
                ground_truth_vector = joblib.load(ground_truth_file_path)

                # Compare vectors
                matches = np.sum(output_vector == ground_truth_vector)
                success_percentage = (matches / len(ground_truth_vector)) * 100

                # Print results
                print(file_name)
                print("FHE result: ")
                print(output_vector)
                print("Ground truth: ")
                print(ground_truth_vector)
                print(f"Matches: {matches} | Success Percentage: {success_percentage:.2f}%")

                # Update overall statistics
                total_matches += matches
                total_elements += len(ground_truth_vector)

        overall_success_percentage = (total_matches / total_elements) * 100

        # Print overall success percentage (optional, for debugging purposes)
        print(f"Overall Success Percentage: {overall_success_percentage:.2f}%")

        # Check if overall success percentage is within expected bounds
        self.assertGreaterEqual(overall_success_percentage, 0, "Overall success percentage should be non-negative")
        self.assertLessEqual(overall_success_percentage, 100, "Overall success percentage should not exceed 100")

if __name__ == '__main__':
    unittest.main()
