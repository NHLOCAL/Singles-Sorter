import matplotlib.pyplot as plt
import json

# Read the iteration data from the JSON file
with open('iteration_data.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

# Extract the Y-axis values
y_values = [data[str(key)]['ner'] for key in sorted(map(int, data.keys()))]

# Create the plot
plt.figure(figsize=(10, 6))
plt.plot(range(len(data)), y_values, marker='o', linestyle='-', color='b')

# Add labels and title
plt.xlabel('Iteration')
plt.ylabel('NER Loss')
plt.title('NER Loss per Iteration')

# Display the plot
plt.grid(True)
plt.savefig('ner_loss_graph.png')
plt.show()