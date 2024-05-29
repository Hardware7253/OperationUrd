import pcbnew

def repeat_selection(schematic, n):
    # Get selected items
    selection = schematic.GetDrawings()
    if not selection:
        print("No items selected.")
        return
    
    # Loop through selected items
    for item in selection:
        if isinstance(item, pcbnew.SCH_TEXT):
            # Increment the first number in the label
            label = item.GetText()
            parts = label.split('_')
            if len(parts) >= 2:
                try:
                    num = int(parts[1])
                    new_label = f"{parts[0]}_{num + 1}"
                    item.SetText(new_label)
                except ValueError:
                    print(f"Invalid label format: {label}")
    
    # Duplicate selected items n times
    for _ in range(n - 1):
        schematic.CopyAndPaste(selection)

def main():
    # Initialize KiCad
    pcbnew.Refresh()
    schematic = pcbnew.GetBoard()
    
    # Get user input for repetition count
    n = int(input("Enter the number of repetitions: "))
    
    # Repeat selection
    repeat_selection(schematic, n)
    
    # Refresh display
    pcbnew.Refresh()

# Run the script
main()

