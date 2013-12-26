#include "swbuilding.h"

#include <stdlib.h>
#include <math.h>

#define PANEL_N_COLS 3

void swbuilding_init(swbuilding_t* w)
{
	subwindow_init(&w->w, L"Building", 1024-SW_WIDTH*3, 0);
}

void swbuilding_exit(swbuilding_t* w)
{
	subwindow_exit(&w->w);
}

void swbuilding_draw(swbuilding_t* w, game_t* g)
{
	if (!subwindow_draw(&w->w, g->g))
		return;

	building_t* b = g->player->inBuilding;
	if (b == NULL)
		return;

	static sfText* text = NULL;
	if (text == NULL)
	{
		sfColor color = {255, 255, 255, 255};

		text = sfText_create();
		sfText_setFont         (text, g->g->font);
		sfText_setCharacterSize(text, 15);
		sfText_setColor        (text, color);
	}

	sfVector2f pos = {w->w.x + 20, w->w.y + 50};

	sfText_setPosition(text, pos);
	sfText_setUnicodeString(text, (sfUint32*) b->t->name);
	sfRenderWindow_drawText(g->g->render, text, NULL);
	pos.y += 20;

	kindOf_building_t* t = b->t;

	if (t->make.n_res != 0)
	{
		component_t* c = &t->make.res[0];
		if (c->is_item)
			exit(1);

		const wchar_t* action = t->make.n_req == 0 ? L"Harvest" : L"Transform to";
		const wchar_t* name   = g->u->materials[c->id].name;
		wchar_t buffer[1024];
		swprintf(buffer, 1024, L"%ls %ls", action, name);

		sfText_setPosition(text, pos);
		sfText_setUnicodeString(text, (sfUint32*) buffer);
		sfRenderWindow_drawText(g->g->render, text, NULL);
	}

	pos.y += 20;
	for (int i = 0; i < t->n_items; i++)
	{
		pos.y += 20;

		transform_t* tr = &t->items[i];
		if (tr->n_res == 0)
			continue;

		component_t* c = &tr->res[0];
		if (!c->is_item)
			exit(1);

		wchar_t* name = g->u->items[c->id].name;
		wchar_t buffer[1024];
		if (i == b->item_current)
			swprintf(buffer, 1024, L"%ls (%i%%)", name, (int) floor(100*b->item_progress));
		else
			swprintf(buffer, 1024, L"%ls", name);

		sfText_setPosition(text, pos);
		sfText_setUnicodeString(text, (sfUint32*) buffer);
		sfRenderWindow_drawText(g->g->render, text, NULL);
	}

//	sfText_destroy(text); // TODO
}

char swbuilding_catch(swbuilding_t* w, game_t* g, float x, float y, int t)
{
	if (!w->w.visible)
		return 0;

	building_t* b = g->player->inBuilding;
	if (b == NULL)
		return 0;

	if (t != sfMouseLeft)
		return 0;

	static sfText* text = NULL;
	if (text == NULL)
	{
		text = sfText_create();
		sfText_setFont         (text, g->g->font);
		sfText_setCharacterSize(text, 18);
	}

	sfVector2f pos = {w->w.x + 20, w->w.y + 90};
	for (int i = 0; i < b->t->n_items; i++)
	{
		pos.y += 20;

		transform_t* tr = &b->t->items[i];
		if (tr->n_res == 0)
			continue;

		component_t* c = &tr->res[0];
		if (!c->is_item)
			exit(1);

		wchar_t* name = g->u->items[c->id].name;
		sfText_setPosition(text, pos);
		sfText_setUnicodeString(text, (sfUint32*) name);

		sfFloatRect rect = sfText_getGlobalBounds(text);
		if (!sfFloatRect_contains(&rect, x, y))
			continue;

		if (transform_check(tr, &g->player->inventory))
		{
			b->item_current = i;
			b->item_progress = 0;
		}

		return 1;
	}

	return 0;
}
