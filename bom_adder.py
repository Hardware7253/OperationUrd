"""Takes seperate kicad bom files from multiple projects within the same root directory, and combines them.
    This is so an accurate number of parts can be ordered for multiple designs containing duplicate parts."""

OUTPUT_BOM_CSV_FORMAT = '"Comment";"Designator";"Footprint";"JLCPCB Part #";"Mouser Part #";"Quantity"'
DEFAULT_BOM_CSV_FORMAT = '"Id";"Designator";"Footprint";"Quantity";"Designation";"Supplier and ref";'

DEFAULT_BOM_VALUE_INDEX = 3
OUTPUT_BOM_VALUE_INDEX = 5

KEY_LENGTH = 5 # How many different data points from the boms are contained in the bom dictionary key

# How output bom column indices map to default bom column indices
# This list only needs to contain indices that are remapped from the default bom, unused indices don't need to be included
OUTPUT_BOM_MAP = [4, 1, 2]

# Column indices in the output bom, which aren't present in the default bom
# These unused indices can be populated from a sample bom, where all other fields (which are a part of the bom dictionary key) match
ID_IGNORE_INDEX = 1 # Ignore designators as they aren't important for identifying a part
DEFAULT_UNUSED_INDICES = [3, 4]

def read_bom_to_dict(bom_dict, bom_path):
    """Reads a bom, and adds contents to a bom dictionary"""
    input_bom_value_index = 0

    with open(bom_path) as infile:
        contents = infile.read()
        format_line = ""
        for i, line in enumerate(contents.splitlines()):
            if i == 0: 
                format_line = line # Get the csv format line

                if format_line == DEFAULT_BOM_CSV_FORMAT:
                    input_bom_value_index = DEFAULT_BOM_VALUE_INDEX
                else:
                    input_bom_value_index = OUTPUT_BOM_VALUE_INDEX

            else:
                
                info = line.split(';')
                value = int(info[input_bom_value_index])

                # Remap and format info
                # Join seperate items into a comma seperated string for the dictionary key
                if format_line == DEFAULT_BOM_CSV_FORMAT:
                    info = remap_list(info, OUTPUT_BOM_MAP, "", KEY_LENGTH)
                else:
                    info.pop(OUTPUT_BOM_VALUE_INDEX)
                    
                key = ';'.join([data for data in info])

                if key in bom_dict:
                    bom_dict[key] += value
                else:
                    bom_dict[key] = value

def populate_unused_indices(sample_bom_id_dict, bom_dict, populate_indices):
    """Populates unused indices in a bom dictionary from a sample
        indices refers to indices of a list after each key is split"""

    updated_dict = dict()
    for key, value in bom_dict.items():
        id_key, id_value = convert_to_id(key, value)

        data = key.split(";")

        if id_key in sample_bom_id_dict:
            for i, index in enumerate(reversed(DEFAULT_UNUSED_INDICES)):
                data[index] = sample_bom_id_dict[id_key][i]

        new_key = ';'.join([element for element in data])
        updated_dict[new_key] = value
    return updated_dict


def create_id_dictionary(bom_dict):
    """Returns a dictionary where the information needed to id a part is the key, and the part number(s) and quantity are the value"""

    output_dict = dict()

    for key, value in bom_dict.items():
        new_key, new_value = convert_to_id(key, value)
        output_dict[new_key] = new_value

    return output_dict

def convert_to_id(key, value):
    """Converts a normal bom dictionary key and value, to an id bom dictionary key and value"""
    data = key.split(";")

    id_data = []
    value_data = [value]
    for i, element in enumerate(data):
        if i in DEFAULT_UNUSED_INDICES:
            value_data.insert(0, element)
        else:
            if i == ID_IGNORE_INDEX:
                id_data.append("")
            else:
                id_data.append(element)
            
    return(';'.join([data for data in id_data]), value_data)

def convert_from_id(key, value):
    """Converts an id key and value back into a normal bom dictionary key and value"""
    data = key.split(";")
    data_shift_list = data + [""] * (KEY_LENGTH - len(DEFAULT_UNUSED_INDICES) - 1)
    converted_key_list = [""] * KEY_LENGTH

    value_index = len(DEFAULT_UNUSED_INDICES) - 1
    for i in range(len(converted_key_list)):
        if i != ID_IGNORE_INDEX and i in DEFAULT_UNUSED_INDICES:

            converted_key_list[i] = value[value_index]
            value_index -= 1

            data_shift_list = [data_shift_list[-1]] + data_shift_list[:-1]
        else:
            converted_key_list[i] = data_shift_list[i]

    return (';'.join([element for element in converted_key_list]), int(value[-1]))
        

def remap_list(input_list, list_map, default, output_length):
    """Returns an output list with it's indices remapped"""
    new_list = []

    for i in range(output_length):
        if i < len(list_map):
            new_list.append(input_list[list_map[i]])
        else:
            new_list.append(default)
    return new_list

def output_formatted_bom(bom_dict, output_path):
    """Takes a bom dictionary and outputs a correctly formatted csv"""

    with open(output_path, "w") as outfile:
        outfile.write(f"{OUTPUT_BOM_CSV_FORMAT}\n")

        for key, value in bom_dict.items():
            line_list = key.split(";")
            line_list.insert(OUTPUT_BOM_VALUE_INDEX, str(value))
            
            line = ';'.join([element for element in line_list])
            outfile.write(f"{line}\n")

def add_to_master_bom(master_bom_dict, bom_dict):
    """Adds a bom_dict to master bom"""

    master_bom_id_dict = create_id_dictionary(bom_dict)
    bom_id_dict = create_id_dictionary(bom_dict)

    for key, value in bom_id_dict.items():
        master_key, master_value = convert_from_id(key, value)
        if master_key in master_bom_dict:
            master_bom_dict[master_key] += master_value
        else:
            master_bom_dict[master_key] = master_value


def main():
    """Program entry point, path settings go here"""

    # It is assumed that the file name is in the format "project_name".csv
    project_names = ["nixie_board", "power_supply", "microcontroller", "toggle_switch_breakout", "mechanical_switch_breakout"]
    sub_directory = "manufacture" # Sub directory which bom csv is located in

    sample_bom_path = "master_bom.csv"
    sample_bom_dict = dict()
    sample_bom_id_dict = dict()
    use_sample_bom = True

    if use_sample_bom:
        read_bom_to_dict(sample_bom_dict, sample_bom_path)
        sample_bom_id_dict = create_id_dictionary(sample_bom_dict)

    master_bom_dict = dict()
    for name in project_names:
        bom_dict = dict()

        bom_path = f"{name}/{sub_directory}/{name}.csv"

        # Add bom to local bom dict
        read_bom_to_dict(bom_dict, bom_path)

        # Populate unused indices (part numbers) if applicable
        if use_sample_bom:
            bom_dict = populate_unused_indices(sample_bom_id_dict, bom_dict, DEFAULT_UNUSED_INDICES)

        # Reformat bom
        output_formatted_bom(bom_dict, bom_path)

        # Add updated bom to master bom dict
        add_to_master_bom(master_bom_dict, bom_dict)

    # Write master bom
    output_master_bom_path = "master_bom.csv"
    output_formatted_bom(master_bom_dict, output_master_bom_path)

main()
