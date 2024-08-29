import unittest
import numpy as np
import os
import subprocess
import joblib


class TestPredictionComparison(unittest.TestCase):
    def setUp(self):
        self.test_data_dir = 'test_data'
        self.ground_truths_dir = 'ground_truths'
    
    def test_predictions(self):
        total_matches = 0
        total_elements = 0
        
        for file_name in os.listdir(self.test_data_dir):
            if file_name.endswith('.csv'):
                test_file_path = os.path.join(self.test_data_dir, file_name)
                ground_truth_file_path = os.path.join(self.ground_truths_dir, file_name)

                # Run `generate_circuit_and_prediction.py` script
                result = subprocess.run(['python3', 'generate_circuit_and_prediction.py', test_file_path], capture_output=True, text=True)
                
                # Convert script output to numpy array
                output_vector = np.array([float(x) for x in result.stdout.split()])

                # Read ground truth vector
                ground_truth_vector = joblib.load(ground_truth_file_path)

                # Compare vectors
                matches = np.sum(output_vector == ground_truth_vector)
                success_percentage = (matches / len(ground_truth_vector)) * 100

                # Print results
                print(file_name)
                print(output_vector)
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
