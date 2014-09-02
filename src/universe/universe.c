/*\
 *  Role playing, management and strategy game
 *  Copyright (C) 2013-2014 Quentin SANTOS
 *
 *  This program is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  This program is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with this program.  If not, see <http://www.gnu.org/licenses/>.
\*/

#include "universe.h"

#include <stdlib.h>
#include <stdio.h>
#include <string.h>

#include "../mem.h"
#include "../string.h"
#include "../file.h"

static void rec_find(universe_t* u, game_t* g, const char* path)
{
	size_t i = 0;
	FOREACH_FILE(path, i++);

	if (i != 0)
	{
		u->characters = CREALLOC(u->characters, kindOf_character_t, u->n_characters+i);
		FOREACH_FILE(path,
			size_t len = strlen(path);
			if (len < 4 || strcmp(path+len-4, ".png"))
				continue;
			kindOf_character_t* c = &u->characters[u->n_characters++];
			kindOf_character_init(c);
			c->sprite = assets_spriteId(g->a, path);
			if (g->s->verbosity >= 2)
				fprintf(stderr, "Loaded '%s'\n", path);
		);
	}

	FOREACH_DIR(path, rec_find(u, g, path));
}

void universe_init(universe_t* u, game_t* g)
{
	// load bots
	u->n_bots = 0;
	FOREACH_FILE("data/bots/", u->n_bots++);
	u->bots = CALLOC(ai_t, u->n_bots);
	for (size_t i = 0; i < u->n_bots; i++)
		ai_init(&u->bots[i]);
	size_t i = 0;
	FOREACH_FILE("data/bots/",
		if (g->s->verbosity >= 2)
			fprintf(stderr, "Parsing bot '%s'\n", path);
		ai_load(&u->bots[i++], path)
	);

	if (g->s->verbosity >= 1)
		fprintf(stderr, "Loaded% 4i bots\n", (int) u->n_bots);

	// load characters
	u->n_characters = 0;
	u->characters = NULL;
	rec_find(u, g, "data/characters/");
	if (g->s->verbosity >= 1)
		fprintf(stderr, "Loaded% 4i characters\n", (int) u->n_characters);

	u->tmp_materials = NULL;
	u->tmp_items = NULL;

	// parse configuration files
	cfg_ini_t ini;
	cfg_ini_init(&ini);
	FOREACH_FILE("data/cfg/",
		if (g->s->verbosity >= 2)
			fprintf(stderr, "Parsing '%s'\n", path);
		cfg_ini_parse(&ini, path)
	);

	// init special skills
	cfg_group_t*   gr      = cfg_ini_group(&ini, "Competences");
	cfg_section_t* s       = &gr->sections[0];
	cfg_array_t*   sskills = cfg_getArray(s, "CompetencesSpeciales");
	u->n_skills = N_SPECIAL_SKILLS;
	u->skills   = CALLOC(kindOf_skill_t, u->n_skills);
	for (size_t i = 0; i < N_SPECIAL_SKILLS && i < sskills->n; i++)
	{
		char* v = sskills->v[i];
		kindOf_skill_t* k = &u->skills[i];
		kindOf_skill_init(k);
		k->name = v == NULL ? NULL : strdup(v);
	}

	// apply rest of configuration
	universe_init_events     (u, g->a, cfg_ini_group(&ini, "Effet"));
	universe_init_materials  (u, g->a, cfg_ini_group(&ini, "Ressource"));
	universe_init_projectiles(u, g->a, cfg_ini_group(&ini, "Projectile"));
	universe_init_mines      (u, g->a, cfg_ini_group(&ini, "TerrainRessource"));
	universe_init_iskills    (u, g->a, cfg_ini_group(&ini, "CompetenceObjet"));
	universe_init_items      (u, g->a, cfg_ini_group(&ini, "Objet"));
	universe_init_buildings  (u, g->a, cfg_ini_group(&ini, "Batiment"));

	if (g->s->verbosity >= 1)
	{
		fprintf(stderr, "Loaded% 4i kinds of event\n",      (int) u->n_events);
		fprintf(stderr, "Loaded% 4i kinds of material\n",   (int) u->n_materials);
		fprintf(stderr, "Loaded% 4i kinds of mine\n",       (int) u->n_mines);
		fprintf(stderr, "Loaded% 4i kinds of item\n",       (int) u->n_items);
		fprintf(stderr, "Loaded% 4i kinds of item skill\n", (int) u->n_iskills);
		fprintf(stderr, "Loaded% 4i kinds of building\n",   (int) u->n_buildings);
	}

	cfg_ini_exit(&ini);

	for (size_t i = 0; i < u->n_items; i++)
		transform_exit(&u->tmp_items[i]);
	free(u->tmp_items);

	for (size_t i = 0; i < u->n_materials; i++)
		transform_exit(&u->tmp_materials[i]);
	free(u->tmp_materials);

	for (size_t i = 0; i < N_STATUSES; i++)
		kindOf_status_init(&u->statuses[i]);

	u->statuses[ST_HEALTH] .name = "Santé";
	u->statuses[ST_STAMINA].name = "Énergie";
	u->statuses[ST_MORAL]  .name = "Moral";
	u->statuses[ST_MANA]   .name = "Magie";
	u->statuses[ST_ATTACK] .name = "Attaque";
	u->statuses[ST_DEFENSE].name = "Défense";

	u->n_categories = 11;
	u->categories = CALLOC(kindOf_category_t, u->n_categories);
	u->categories[ 0].name = "Arme à une main";
	u->categories[ 1].name = "Arme à deux mains";
	u->categories[ 2].name = "Armure";
	u->categories[ 3].name = "Casque";
	u->categories[ 4].name = "Chaussures";
	u->categories[ 5].name = "Dos";
	u->categories[ 6].name = "Anneau à une main";
	u->categories[ 7].name = "Anneau à 2 mains";
	u->categories[ 8].name = "Bras";
	u->categories[ 9].name = "Colliers";
	u->categories[10].name = "Monture";

	u->n_slots = 11;
	u->slots = CALLOC(kindOf_slot_t, u->n_slots);
	u->slots[ 0] = (kindOf_slot_t){"Main droite",       0};
	u->slots[ 1] = (kindOf_slot_t){"Main gauche",       0};
	u->slots[ 2] = (kindOf_slot_t){"Torse",             2};
	u->slots[ 3] = (kindOf_slot_t){"Tête",              3};
	u->slots[ 4] = (kindOf_slot_t){"Pieds",             4};
	u->slots[ 5] = (kindOf_slot_t){"Dos",               5};
	u->slots[ 6] = (kindOf_slot_t){"Annulaire droit",   6};
	u->slots[ 7] = (kindOf_slot_t){"Annulaire gauche",  6};
	u->slots[ 8] = (kindOf_slot_t){"Mains",             8};
	u->slots[ 9] = (kindOf_slot_t){"Cou",               9};
	u->slots[10] = (kindOf_slot_t){"Monture",          10};
}

void universe_exit(universe_t* u)
{
	/* TODO
	for (size_t i = 0; i < u->n_slots; i++)
		kindOf_slot_exit(&u->slots[i]);
	*/
	free(u->slots);

	/* TODO
	for (size_t i = 0; i < u->n_categories; i++)
		kindOf_category_exit(&u->categories[i]);
	*/
	free(u->categories);

	/* TODO
	for (size_t i = 0; i < N_STATUSES; i++)
		kindOf_status_exit(&u->statuses[i]);
	*/

	free(u->iskills);

	for (size_t i = 0; i < u->n_skills; i++)
		kindOf_skill_exit(&u->skills[i]);
	free(u->skills);

	for (size_t i = 0; i < u->n_buildings; i++)
		kindOf_building_exit(&u->buildings[i]);
	free(u->buildings);

	for (size_t i = 0; i < u->n_mines; i++)
		kindOf_mine_exit(&u->mines[i]);
	free(u->mines);

	for (size_t i = 0; i < u->n_items; i++)
		kindOf_item_exit(&u->items[i]);
	free(u->items);

	for (size_t i = 0; i < u->n_projectiles; i++)
		kindOf_projectile_exit(&u->projectiles[i]);
	free(u->projectiles);

	for (size_t i = 0; i < u->n_materials; i++)
		kindOf_material_exit(&u->materials[i]);
	free(u->materials);

	for (size_t i = 0; i < u->n_characters; i++)
		kindOf_character_exit(&u->characters[i]);
	free(u->characters);

	for (size_t i = 0; i < u->n_events; i++)
		kindOf_event_exit(&u->events[i]);
	free(u->events);

	for (size_t i = 0; i < u->n_bots; i++)
		ai_exit(&u->bots[i]);
	free(u->bots);
}

void universe_init_events(universe_t* u, assets_t* a, cfg_group_t* gr)
{
	if (gr == NULL)
		return;

	u->n_events = gr->n_sections;
	u->events   = CALLOC(kindOf_event_t, u->n_events);
	u->event_destroyBuilding = 23; // TODO: load from Parametres_Effets.ini
	for (size_t i = 0; i < u->n_events; i++)
	{
		cfg_section_t*  s = &gr->sections[i];
		kindOf_event_t* e = &u->events[i];
		kindOf_event_init(e);

		// graphics
		char* image_file = cfg_getString(s, "Fichier");
		int n_steps = cfg_getInt(s, "NombreEtapes");
		if (image_file != NULL && n_steps > 0)
			kindOf_event_sprite(e, a, image_file, n_steps);
		free(image_file);

		float duration = cfg_getFloat(s, "DureeEtape");
		duration /= 10;
		duration *= n_steps;
		e->duration = duration;

		// sound
		char* sound_file = cfg_getString(s, "Son");
		if (sound_file != NULL)
			kindOf_event_sound(e, sound_file);
	}
}

void universe_init_materials(universe_t* u, assets_t* a, cfg_group_t* gr)
{
	if (gr == NULL)
		return;

	u->n_materials   = gr->n_sections;
	u->materials     = CALLOC(kindOf_material_t, u->n_materials);
	u->tmp_materials = CALLOC(transform_t,       u->n_materials);
	u->skills = CREALLOC(u->skills, kindOf_skill_t, u->n_skills + u->n_materials);
	for (size_t i = 0; i < u->n_materials; i++)
	{
		cfg_section_t*     s = &gr->sections[i];
		kindOf_material_t* m = &u->materials[i];
		transform_t*       t = &u->tmp_materials[i];

		kindOf_material_init(m);
		transform_init(t);
		transform_res(t, MATERIAL, i, 1.0);
		t->rate = cfg_getFloat(s, "VitesseExtraction");

		int   id = cfg_getInt(s, "TypeMatierePremiere") - 1;
		float am = cfg_getFloat(s, "QuantiteMatierePremiere");
		if (id >= (ssize_t) u->n_materials)
		{
			fprintf(stderr, "Invalid requisite material id '%i' at '%s_%s'\n",
				id, gr->name, s->name);
			exit(1);
		}
		if (id >= 0)
			transform_req(t, MATERIAL, id, am);

		m->name                 = cfg_getString(s, "Nom");
		m->edible               = cfg_getInt   (s, "Mangeable");
		m->eatBonus[ST_HEALTH]  = cfg_getFloat (s, "GainVie");
		m->eatBonus[ST_STAMINA] = cfg_getFloat (s, "GainEnergie");
		m->eatBonus[ST_MORAL]   = cfg_getFloat (s, "GainMoral");
		m->eatBonus[ST_MANA]    = cfg_getFloat (s, "GainMagie");
		m->price                = cfg_getFloat (s, "PrixVente");

		size_t skill = u->n_skills++;
		kindOf_skill_t* k = &u->skills[skill];
		kindOf_skill_init(k);
		k->name = cfg_getString(s, "NomCompetence");
		m->skill = skill;

		char* icon_file  = cfg_getString(s, "Image");
		int   icon_index = cfg_getInt   (s, "SpriteIndex") - 1;
		if (icon_file != NULL && icon_index >= 0)
			kindOf_material_icon(m, a, icon_file, icon_index);
		else
			fprintf(stderr, "Missing icon for '%s_%s'\n", gr->name, s->name);
		free(icon_file);
	}
}

void universe_init_projectiles(universe_t* u, assets_t* a, cfg_group_t* gr)
{
	if (gr == NULL)
		return;

	u->n_projectiles = gr->n_sections;
	u->projectiles   = CALLOC(kindOf_projectile_t, u->n_projectiles);
	for (size_t i = 0; i < u->n_projectiles; i++)
	{
		cfg_section_t* s = &gr->sections[i];
		kindOf_projectile_t* p = &u->projectiles[i];
		kindOf_projectile_init(p);

		const char* image = cfg_getString(s, "CheminApparence");
		if (image != NULL)
			kindOf_projectile_sprite(p, a, image);
		p->event = cfg_getInt(s, "EffetAttaque") - 1;
	}
}

void universe_init_mines(universe_t* u, assets_t* a, cfg_group_t* gr)
{
	(void) a;

	if (gr == NULL)
		return;

	u->n_mines = gr->n_sections;
	u->mines   = CALLOC(kindOf_mine_t, u->n_mines);
	for (size_t i = 0; i < u->n_mines; i++)
	{
		cfg_section_t* s = &gr->sections[i];
		kindOf_mine_t* m = &u->mines[i];
		kindOf_mine_init(m, i);

		m->name = cfg_getString(s, "Nom");
		int id = cfg_getInt(s, "TypeRessource") - 1;
		if (id < 0 || (size_t) id >= u->n_materials)
		{
			fprintf(stderr, "Invalid requisite material id '%i' at '%s_%s'\n",
				id, gr->name, s->name);
			exit(1);
		}
		transform_copy(&m->harvest, &u->tmp_materials[id]);
	}
}

void universe_init_iskills(universe_t* u, assets_t* a, cfg_group_t* gr)
{
	(void) a;

	if (gr == NULL)
		return;

	u->n_iskills = gr->n_sections;
	u->iskills = CALLOC(int, u->n_iskills);
	u->skills = CREALLOC(u->skills, kindOf_skill_t, u->n_skills + u->n_iskills);
	for (size_t i = 0; i < u->n_iskills; i++)
	{
		cfg_section_t*  s = &gr->sections[i];

		size_t skill = u->n_skills++;
		kindOf_skill_t* k = &u->skills[skill];
		kindOf_skill_init(k);
		k->name = cfg_getString(s, "NomFR");
		u->iskills[i] = skill;
	}
}

void universe_init_items(universe_t* u, assets_t* a, cfg_group_t* gr)
{
	if (gr == NULL)
		return;

	u->n_items   = gr->n_sections;
	u->items     = CALLOC(kindOf_item_t, u->n_items);
	u->tmp_items = CALLOC(transform_t,   u->n_items);
	for (size_t i = 0; i < u->n_items; i++)
	{
		cfg_section_t* s = &gr->sections[i];
		kindOf_item_t* it = &u->items[i];
		transform_t*   t  = &u->tmp_items[i];

		kindOf_item_init(it);
		it->name = cfg_getString(s, "Nom");
		if (it->name == NULL)
			it->name = cfg_getString(s, "NomFR");
		it->category = cfg_getInt(s, "Categorie");

		int iskill = cfg_getInt(s, "Competence") - 1;
		it->skill = u->iskills[iskill];
		it->event = cfg_getInt(s, "EffetAttaque") - 1;

		it->range      = cfg_getFloat(s, "BonusPorteeAttaque");
		it->projectile = cfg_getInt  (s, "LanceProjectile") - 1;
		float delay    = cfg_getFloat(s, "DureeRechargeAttaque");
		if (delay != 0)
			it->reloadDelay = delay;

		transform_init(t);
		transform_res(t, ITEM, i, 1.0);
		t->rate = 100. / cfg_getInt(s, "DureeFabrication");

		cfg_array_t* cost = cfg_getArray(s, "PrixRessources");
		if (cost != NULL)
		for (size_t i = 0; i < cost->n; i++)
		{
			char* v = cost->v[i];
			if (v == NULL)
				continue;
			if (i >= u->n_materials)
			{
				fprintf(stderr, "Invalid requisite material id '%u' at '%s_%s'\n",
					(unsigned) i, gr->name, s->name);
				exit(1);
			}
			transform_req(t, MATERIAL, i, atof(v));
		}

		it->price = cfg_getFloat(s, "PrixVente");

		cfg_array_t* sboni = cfg_getArray(s, "BonusCompetencesSpeciales");
		if (sboni != NULL)
		for (size_t i = 0; i < sboni->n; i++)
		{
			char* v = sboni->v[i];
			if (v == NULL)
				continue;
			if (i >= N_SPECIAL_SKILLS)
			{
				fprintf(stderr, "Invalid special skill id '%u' at '%s_%s'\n",
					(unsigned) i, gr->name, s->name);
				exit(1);
			}
			int skill = i;
			effect_skill(&it->effect, skill, atof(v));
		}

		it->effect.status_bonus[ST_HEALTH]  = cfg_getFloat(s, "BonusVie");
		it->effect.status_bonus[ST_STAMINA] = cfg_getFloat(s, "BonusEnergie");
		it->effect.status_bonus[ST_MORAL]   = cfg_getFloat(s, "BonusMoral");
		it->effect.status_bonus[ST_MANA]    = cfg_getFloat(s, "BonusMagie");
		it->effect.status_bonus[ST_ATTACK]  = cfg_getFloat(s, "BonusAttaque");
		it->effect.status_bonus[ST_DEFENSE] = cfg_getFloat(s, "BonusDefense");

		it->effect.max_material = cfg_getFloat(s, "BonusMaxRessources");
		it->effect.armor        = cfg_getFloat(s, "BonusArmure");

		float walking = cfg_getFloat(s, "BonusDeplacement");
		if (walking != 0)
			effect_skill(&it->effect, SK_WALK, walking);

		// TODO
		float attack = cfg_getFloat(s, "BonusAttaque");
		if (attack != 0)
			effect_skill(&it->effect, SK_ATTACK, attack);

		cfg_array_t* mboni = cfg_getArray(s, "BonusCompetencesRessources");
		if (mboni != NULL)
		for (size_t i = 0; i < mboni->n; i++)
		{
			char* v = mboni->v[i];
			if (v == NULL)
				continue;
			if (i >= u->n_materials)
			{
				fprintf(stderr, "Invalid material skill id '%u' at '%s_%s'\n",
					(unsigned) i, gr->name, s->name);
				exit(1);
			}
			int skill = u->materials[i].skill;
			effect_skill(&it->effect, skill, atof(v));
		}

		cfg_array_t* iboni = cfg_getArray(s, "BonusCompetencesObjets");
		if (iboni != NULL)
		for (size_t i = 0; i < iboni->n; i++)
		{
			char* v = iboni->v[i];
			if (v == NULL)
				continue;
			if (i >= u->n_iskills)
			{
				fprintf(stderr, "Invalid item skill id '%u' at '%s_%s'\n",
					(unsigned) i, gr->name, s->name);
				exit(1);
			}
			int skill = u->iskills[i];
			effect_skill(&it->effect, skill, atof(v));
		}

		it->reqMana = cfg_getFloat(s, "PerteMagie");

		int   id = cfg_getInt(s, "PerteRessourceIndice") - 1;
		float am = cfg_getFloat(s, "PerteRessourceQuantite");
		if (id >= 0)
			transform_req(&it->cost, MATERIAL, id, am);

		char* icon_file  = cfg_getString(s, "Image");
		int   icon_index = cfg_getInt   (s, "SpriteIndex") - 1;
		if (icon_file != NULL && icon_index >= 0)
			kindOf_item_icon(it, a, icon_file, icon_index);
		else
			fprintf(stderr, "Missing icon for '%s_%s'\n", gr->name, s->name);
		free(icon_file);
	}
}

void universe_init_buildings(universe_t* u, assets_t* a, cfg_group_t* gr)
{
	if (gr == NULL)
		return;

	u->n_buildings = gr->n_sections;
	u->buildings   = CALLOC(kindOf_building_t, u->n_buildings);
	for (size_t i = 0; i < u->n_buildings; i++)
	{
		cfg_section_t*     s = &gr->sections[i];
		kindOf_building_t* b = &u->buildings[i];
		kindOf_building_init(b);

		b->name = cfg_getString(s, "Nom");
		if (b->name == NULL)
			b->name = cfg_getString(s, "NomFR");
		b->build.rate = 100. / cfg_getFloat(s, "MaxVie");

		int id = cfg_getInt(s, "RessourceFabrique") - 1;
		if (id >= (ssize_t) u->n_materials)
		{
			fprintf(stderr, "Invalid result material id '%i' at '%s_%s'\n",
				id, gr->name, s->name);
			exit(1);
		}
		if (id >= 0)
			transform_copy(&b->make, &u->tmp_materials[id]);

		char* image_file = cfg_getString(s, "Image");
		int   n_sprites  = cfg_getInt(s, "NombreEtapeFabrication") + 1;
		if (image_file != NULL && n_sprites > 0)
			kindOf_building_sprite(b, a, image_file, n_sprites);
		else
			fprintf(stderr, "Missing image for '%s_%s'\n", gr->name, s->name);
		free(image_file);

		char* button_file  = cfg_getString(s, "ImageBouton");
		int   button_index = cfg_getInt   (s, "SpriteBoutonIndex") - 1;
		if (button_file != NULL && button_index >= 0)
			kindOf_building_button(b, a, button_file, button_index);
		else
			fprintf(stderr, "Missing button for '%s_%s'\n", gr->name, s->name);
		free(button_file);

		b->door_dx = cfg_getFloat(s, "EntreePositionX");
		b->door_dy = cfg_getFloat(s, "EntreePositionY") - b->height/2 + 16;

		cfg_array_t* cost = cfg_getArray(s, "PrixRessources");
		if (cost != NULL)
		for (size_t i = 0; i < cost->n; i++)
		{
			char* v = cost->v[i];
			if (v == NULL)
				continue;
			if (i >= u->n_materials)
			{
				fprintf(stderr, "Invalid requisite material id '%u' at '%s_%s'\n",
					(unsigned) i, gr->name, s->name);
				exit(1);
			}
			transform_req(&b->build, MATERIAL, i, atof(v));
		}

		cfg_array_t* items = cfg_getArray(s, "ObjetsFabriques");
		if (items != NULL)
		for (size_t i = 0; i < items->n; i++)
		{
			char* v = items->v[i];
			if (v == NULL)
				continue;

			int i = kindOf_building_newItem(b);
			int id = strtol(v, NULL, 0) - 1;
			if (id < 0 || (size_t) id >= u->n_items)
			{
				fprintf(stderr, "Invalid avalaible item id '%i' at '%s_%s'\n",
					id, gr->name, s->name);
				exit(1);
			}
			transform_copy(&b->items[i], &u->tmp_items[id]);
		}
	}
}

kindOf_mine_t* universe_mineFor(universe_t* u, char is_item, int id)
{
	for (size_t i = 0; i < u->n_mines; i++)
	{
		kindOf_mine_t* t = &u->mines[i];
		if (transform_is_res(&t->harvest, is_item, id) >= 0)
			return t;
	}
	return NULL;
}

kindOf_building_t* universe_buildFor(universe_t* u, char is_item, int id)
{
	for (size_t i = 0; i < u->n_buildings; i++)
	{
		kindOf_building_t* b = &u->buildings[i];
		transform_t* tr = kindOf_building_canMake(b, is_item, id);
		if (tr != NULL)
			return b;
	}
	return NULL;
}
