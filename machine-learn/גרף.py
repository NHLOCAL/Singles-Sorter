import matplotlib.pyplot as plt

# Given data
data = {
0: {'ner': 5895.014862785158},
1: {'ner': 4715.0510663798805},
2: {'ner': 4365.938335093422},
3: {'ner': 4175.397387537203},
4: {'ner': 4146.644741721774},
5: {'ner': 3960.034096939464},
6: {'ner': 3956.8994222289334},
7: {'ner': 4013.715788320473},
}

# Extract the Y-axis values
y_values = [data[key]['ner'] for key in sorted(data.keys())]

# Create the plot
plt.figure(figsize=(10, 6))
plt.plot(range(1, len(data) + 1), y_values, marker='o', linestyle='-', color='b')

# Add labels and title
plt.xlabel('Data Point')
plt.ylabel('Value')
plt.title('Graph of the Given Data')

# Display the plot
plt.grid(True)
plt.show()
