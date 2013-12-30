#include "universe.h"

#include <string.h>

#include "../util.h"

universe_t* universe_init(graphics_t* g)
{
	universe_t* u = CALLOC(universe_t, 1);

	u->n_materials = 0;
	u->materials = NULL;

	u->n_items = 0;
	u->items = NULL;

	u->n_mines = 0;
	u->mines = NULL;

	u->n_buildings = 0;
	u->buildings = NULL;

	for (int i = 0; i < N_SPECIAL_SKILLS; i++)
		kindOf_skill_init(&u->sskills[i]);

	u->n_iskills = 0;
	u->iskills = NULL;

	for (int i = 0; i < N_STATUSES; i++)
		kindOf_status_init(&u->statuses[i]);

	u->statuses[ST_HEALTH] .name = L"Santé";
	u->statuses[ST_STAMINA].name = L"Énergie";
	u->statuses[ST_MORAL]  .name = L"Moral";
	u->statuses[ST_MANA]   .name = L"Magie";

	u->n_categories = 11;
	u->categories = CALLOC(kindOf_category_t, u->n_categories);
	u->categories[ 0].name = L"Arme à une main";
	u->categories[ 1].name = L"Arme à deux mains";
	u->categories[ 2].name = L"Armure";
	u->categories[ 3].name = L"Casque";
	u->categories[ 4].name = L"Chaussures";
	u->categories[ 5].name = L"Dos";
	u->categories[ 6].name = L"Anneau à une main";
	u->categories[ 7].name = L"Anneau à 2 mains";
	u->categories[ 8].name = L"Bras";
	u->categories[ 9].name = L"Colliers";
	u->categories[10].name = L"Monture";

	u->n_slots = 11;
	u->slots = CALLOC(kindOf_slot_t, u->n_slots);
	u->slots[ 0] = (kindOf_slot_t){L"Main droite",       0};
	u->slots[ 1] = (kindOf_slot_t){L"Main gauche",       0};
	u->slots[ 2] = (kindOf_slot_t){L"Torse",             2};
	u->slots[ 3] = (kindOf_slot_t){L"Tête",              3};
	u->slots[ 4] = (kindOf_slot_t){L"Pieds",             4};
	u->slots[ 5] = (kindOf_slot_t){L"Dos",               5};
	u->slots[ 6] = (kindOf_slot_t){L"Annulaire droit",   6};
	u->slots[ 7] = (kindOf_slot_t){L"Annulaire gauche",  6};
	u->slots[ 8] = (kindOf_slot_t){L"Main",              8};
	u->slots[ 9] = (kindOf_slot_t){L"Cou",               9};
	u->slots[10] = (kindOf_slot_t){L"Monture",          10};

	// parse configuration files
	u->tmp_materials = NULL;
	u->tmp_items = NULL;

	universe_parse(u, g, "cfg/Parametres_Ressources.ini");
	universe_parse(u, g, "cfg/Parametres_CompetencesObjets.ini");
	universe_parse(u, g, "cfg/Parametres_Objets.ini");
	universe_parse(u, g, "cfg/Parametres_Batiments.ini");
	universe_parse(u, g, "cfg/Competences_Speciales.ini");

	for (int i = 0; i < u->n_items; i++)
		transform_exit(&u->tmp_items[i]);
	free(u->tmp_items);

	for (int i = 0; i < u->n_materials; i++)
		transform_exit(&u->tmp_materials[i]);
	free(u->tmp_materials);

	return u;
}

void universe_exit(universe_t* u)
{
	/* TODO
	for (int i = 0; i < u->n_slots; i++)
		kindOf_slots_exit(&u->slots[i]);
	*/
	free(u->slots);

	/* TODO
	for (int i = 0; i < u->n_categories; i++)
		kindOf_status_exit(&u->categories[i]);
	*/
	free(u->categories);

	/* TODO
	for (int i = 0; i < N_STATUSES; i++)
		kindOf_status_exit(&u->statuses[i]);
	*/

	for (int i = 0; i < u->n_iskills; i++)
		kindOf_skill_exit(&u->iskills[i]);
	free(u->iskills);

	for (int i = 0; i < N_SPECIAL_SKILLS; i++)
		kindOf_skill_exit(&u->sskills[i]);

	for (int i = 0; i < u->n_buildings; i++)
		kindOf_building_exit(&u->buildings[i]);
	free(u->buildings);

	for (int i = 0; i < u->n_mines; i++)
		kindOf_mine_exit(&u->mines[i]);
	free(u->mines);

	for (int i = 0; i < u->n_items; i++)
		kindOf_item_exit(&u->items[i]);
	free(u->items);

	for (int i = 0; i < u->n_materials; i++)
		kindOf_material_exit(&u->materials[i]);
	free(u->materials);

	free(u);
}

void universe_parse(universe_t* u, graphics_t* g, const char* filename)
{
	FILE* f = fopen(filename, "r");
	if (f == NULL)
	{
		fprintf(stderr, "Could not open '%s'\n", filename);
		exit(1);
	}

	char*  line  = NULL;
	size_t nline = 0;

	// 0 = none, 1 = material, 2 = item, 3 = mine,
	// 4 = building, 5 = item skill
	int cur_blck = 0;

	int cur_id = 0;

	char* image_file = NULL;
	int n_sprites = 0;
	char* button_file = NULL;
	int button_index = -1;

	while (1)
	{
		if (getline(&line, &nline, f) <= 0)
			break;

		if (strncmp(line, "[Ressource_", 11) == 0) // material
		{
			cur_blck = 1;
			cur_id = atoi(line+11);
			if (cur_id > u->n_materials)
			{
				u->materials = CREALLOC(u->materials, kindOf_material_t, cur_id);
				u->tmp_materials = CREALLOC(u->tmp_materials, transform_t, cur_id);
				for (int i = u->n_materials; i < cur_id; i++)
				{
					kindOf_material_init(&u->materials[i]);
					transform_init(&u->tmp_materials[i]);
					transform_res(&u->tmp_materials[i], i, 1, 0);
				}
				u->n_materials = cur_id;
			}
			cur_id--;
			continue;
		}
		else if (strncmp(line, "[Objet_", 7) == 0) // item
		{
			cur_blck = 2;
			cur_id = atoi(line + 7);
			if (cur_id > u->n_items)
			{
				u->items = CREALLOC(u->items, kindOf_item_t, cur_id);
				u->tmp_items = CREALLOC(u->tmp_items, transform_t,  cur_id);
				for (int i = u->n_items; i < cur_id; i++)
				{
					kindOf_item_init(&u->items[i], u);
					transform_init(&u->tmp_items[i]);
					transform_res(&u->tmp_items[i], i, 1, 1);
				}
				u->n_items = cur_id;
			}
			cur_id--;
			continue;
		}
		else if (strncmp(line, "[TerrainRessource_", 18) == 0) // mine
		{
			cur_blck = 3;
			cur_id = atoi(line+18);
			if (cur_id > u->n_mines)
			{
				u->mines = CREALLOC(u->mines, kindOf_mine_t, cur_id);
				for (int i = u->n_mines; i < cur_id; i++)
				{
					kindOf_mine_init(&u->mines[i]);
					u->mines[i].id = i;
				}
				u->n_mines = cur_id;
			}
			cur_id--;
			continue;
		}
		else if (strncmp(line, "[Batiment_", 10) == 0) // building
		{
			cur_blck = 4;
			cur_id = atoi(line+10);
			if (cur_id > u->n_buildings)
			{
				u->buildings = CREALLOC(u->buildings, kindOf_building_t, cur_id);
				for (int i = u->n_buildings; i < cur_id; i++)
					kindOf_building_init(&u->buildings[i]);
				u->n_buildings = cur_id;
			}
			cur_id--;

			free(image_file);
			free(button_file);

			image_file = NULL;
			n_sprites = 0;
			button_file = NULL;
			button_index = -1;
			continue;
		}
		else if (strncmp(line, "[CompetenceObjet_", 17) == 0) // item skill
		{
			cur_blck = 5;
			cur_id = atoi(line+17);
			if (cur_id > u->n_iskills)
			{
				u->iskills = CREALLOC(u->iskills, kindOf_skill_t, cur_id);
				for (int i = u->n_iskills; i < cur_id; i++)
					kindOf_skill_init(&u->iskills[i]);
				u->n_iskills = cur_id;
			}
			cur_id--;
			continue;
		}
		// probably another section
		else if (line[0] == '[')
		{
			cur_blck = 0;
		}

		// check if it's an affectation
		char* sep = strchr(line, '=');
		if (sep == NULL)
			continue;

		// identify the variable name
		char* var = line + strspn(line, " \t");
		var[strcspn(var, " \t=")] = 0;

		// identify the value
		sep++;
		char* val = sep + strspn(sep, " \t\"");
		val[strcspn(val, "\r\n\"")] = 0;

		if (cur_blck == 0) // global
		{
			if (strncmp(var, "CompetencesSpeciales", 20) == 0)
			{
				int id = atoi(var+20) - 1;
				u->sskills[id].name = strdupwcs(val);
			}
		}

		if (cur_id < 0)
			continue;

		if (cur_blck == 1) // material
		{
			if (strcmp(var, "Nom") == 0)
			{
				u->materials[cur_id].name = strdupwcs(val);
			}
			else if (strcmp(var, "VitesseExtraction") == 0)
			{
				u->tmp_materials[cur_id].rate = atof(val);
			}
			else if (strcmp(var, "TypeMatierePremiere") == 0)
			{
				int id = atoi(val) - 1;
				if (id >= 0)
					transform_req(&u->tmp_materials[cur_id], id, 0, 0);
			}
			else if (strcmp(var, "QuantiteMatierePremiere") == 0)
			{
				if (u->tmp_materials[cur_id].n_req > 0)
					u->tmp_materials[cur_id].req[0].amount = atof(val);
			}
			else if (strcmp(var, "NomCompetence") == 0)
			{
				u->materials[cur_id].skill.name = strdupwcs(val);
			}
			else if (strcmp(var, "Mangeable") == 0)
			{
				u->materials[cur_id].edible = atoi(val);
			}
			else if (strcmp(var, "GainVie") == 0)
			{
				u->materials[cur_id].eatBonus[ST_HEALTH] = atof(val);
			}
			else if (strcmp(var, "GainEnergie") == 0)
			{
				u->materials[cur_id].eatBonus[ST_STAMINA] = atof(val);
			}
			else if (strcmp(var, "GainMoral") == 0)
			{
				u->materials[cur_id].eatBonus[ST_MORAL] = atof(val);
			}
			else if (strcmp(var, "GainMagie") == 0)
			{
				u->materials[cur_id].eatBonus[ST_MANA] = atof(val);
			}
		}
		else if (cur_blck == 2) // item
		{
			if (strcmp(var, "Nom") == 0 || strcmp(var, "NomFR") == 0)
			{
				u->items[cur_id].name = strdupwcs(val);
			}
			else if (strcmp(var, "Categorie") == 0)
			{
				u->items[cur_id].category = atoi(val);
			}
			else if (strcmp(var, "Competence") == 0)
			{
				u->items[cur_id].skill = atoi(val) - 1;
			}
			else if (strncmp(var, "PrixRessources(", 15) == 0)
			{
				int mat_id = atoi(var + 15) - 1;
				float amount = atof(val);
				transform_req(&u->tmp_items[cur_id], mat_id, amount, 0);
			}
			else if (strcmp(var, "DureeFabrication") == 0)
			{
				u->tmp_items[cur_id].rate = 100.f / atoi(val);
			}
			else if (strncmp(var, "BonusCompetencesSpeciales(", 26) == 0)
			{
				int id = atoi(var + 26) - 1;
				float amount = atof(val);
				u->items[cur_id].bonus_special[id] = amount;
			}
			else if (strncmp(var, "BonusCompetencesRessources(", 27) == 0)
			{
				int id = atoi(var + 27) - 1;
				float amount = atof(val);
				u->items[cur_id].bonus_material[id] = amount;
			}
			else if (strncmp(var, "BonusCompetencesObjets(", 23) == 0)
			{
				int id = atoi(var + 23) - 1;
				float amount = atof(val);
				u->items[cur_id].bonus_item[id] = amount;
			}
		}
		else if (cur_blck == 3) // mine
		{
			if (strcmp(var, "Nom") == 0)
			{
				u->mines[cur_id].name = strdupwcs(val);
			}
			else if (strcmp(var, "TypeRessource") == 0)
			{
				int id = atoi(val) - 1;
				transform_copy(&u->mines[cur_id].harvest, &u->tmp_materials[id]);
			}
		}
		else if (cur_blck == 4) // building
		{
			if (strcmp(var, "Nom") == 0)
			{
				u->buildings[cur_id].name = strdupwcs(val);
			}
			else if (strcmp(var, "Image") == 0)
			{
				image_file = strdup(val);
			}
			else if (strcmp(var, "ImageBouton") == 0)
			{
				button_file = strdup(val);
			}
			else if (strcmp(var, "SpriteBoutonIndex") == 0)
			{
				button_index = atoi(val) - 1;
			}
			else if (strcmp(var, "MaxVie") == 0)
			{
				u->buildings[cur_id].build.rate = 100. / atof(val);
			}
			else if (strncmp(var, "PrixRessources(", 15) == 0)
			{
				int mat_id = atoi(var + 15) - 1;
				float amount = atof(val);
				transform_req(&u->buildings[cur_id].build, mat_id, amount, 0);
			}
			else if (strcmp(var, "RessourceFabrique") == 0)
			{
				int id = atoi(val) - 1;
				transform_copy(&u->buildings[cur_id].make, &u->tmp_materials[id]);
			}
			else if (strncmp(var, "ObjetsFabriques(", 16) == 0)
			{
				int id = atoi(val) - 1;

				kindOf_building_t* b = &u->buildings[cur_id];
				int i = kindOf_building_newItem(b);
				transform_copy(&b->items[i], &u->tmp_items[id]);;
			}
			else if (strcmp(var, "NombreEtapeFabrication") == 0)
			{
				n_sprites = atoi(val);
			}

			if (image_file && n_sprites)
			{
				kindOf_building_sprite(&u->buildings[cur_id], g, image_file, n_sprites+1);

				image_file = NULL;
				n_sprites = 0;
			}
			if (button_file && button_index >= 0)
			{
				kindOf_building_button(&u->buildings[cur_id], g, button_file, button_index);

				button_file = NULL;
				button_index = 0;
			}
		}
		else if (cur_blck == 5) // item skill
		{
			if (strcmp(var, "NomFR") == 0)
			{
				u->iskills[cur_id].name = strdupwcs(val);
			}
		}
	}

	fclose(f);
}
