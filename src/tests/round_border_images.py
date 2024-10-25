import os
from PIL import Image, ImageDraw

def add_rounded_corners(image_path, output_path, radius):
    img = Image.open(image_path).convert("RGBA")
    width, height = img.size
    
    # Create a rounded mask
    rounded_mask = Image.new("L", (width, height), 0)
    draw = ImageDraw.Draw(rounded_mask)
    draw.rounded_rectangle((0, 0, width, height), radius=radius, fill=255)
    
    # Apply the mask to the image
    img.putalpha(rounded_mask)
    
    # Save the output image
    img.save(output_path, format="PNG")

def process_images_in_folder(input_folder, output_folder, radius):
    # Check if the output folder exists, if not, create it
    if not os.path.exists(output_folder):
        os.makedirs(output_folder)

    # Loop through all files in the input folder
    for filename in os.listdir(input_folder):
        if filename.lower().endswith(('.png', '.jpg', '.jpeg')):  # Filter image files
            input_path = os.path.join(input_folder, filename)
            output_path = os.path.join(output_folder, filename)
            add_rounded_corners(input_path, output_path, radius)
            print(f"Processed: {filename}")

# שימוש בקוד
input_folder = r"c:\Users\me\Documents\GitHub\Singles-Sorter\src\core\app\assets"
output_folder = r"c:\Users\me\Documents\GitHub\Singles-Sorter\src\core\app\assets2"
radius = 15  # רמת עיגול הפינות

process_images_in_folder(input_folder, output_folder, radius)