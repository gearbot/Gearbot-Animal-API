import json

with open("uncleaned_cat_facts.json", encoding="utf8") as raw_facts:
    raw_json = json.load(raw_facts)
    
    formatted_facts = []

    id_counter = 0
    for uncleaned_fact in raw_json:
        formatted_facts.append({
            "id": id_counter,
            "fact": uncleaned_fact["fact"]
        })
        id_counter += 1

    with open("cat_facts.json", "w") as cleaned_json:
        json.dump(formatted_facts, cleaned_json)
