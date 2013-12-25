#include "swinventory.h"

#include <math.h>

#define PANEL_N_COLS 3

void swinventory_init(swinventory_t* w)
{
	subwindow_init(&w->w, L"Inventory", 1024-SW_WIDTH*2, 0);
}

void swinventory_exit(swinventory_t* w)
{
	subwindow_exit(&w->w);
}

void swinventory_draw(swinventory_t* w, game_t* g)
{
	subwindow_draw(&w->w, g->g);

	sfText* text = NULL;
	if (text == NULL)
	{
		sfColor color = {255, 255, 255, 255};

		text = sfText_create();
		sfText_setFont         (text, g->g->font);
		sfText_setCharacterSize(text, 15);
		sfText_setColor        (text, color);
	}

	sfVector2f pos = {w->w.x + 20, w->w.y + 50};

	for (int i = 0; i < g->u->n_materials; i++)
	{
		const wchar_t* name = g->u->materials[i].name;
		int amount = floor(g->player->inventory.materials[i]);

		if (amount == 0)
			continue;

		wchar_t buffer[1024];
		swprintf(buffer, 1024, L"%ls: %i", name, amount);

		sfText_setPosition(text, pos);
		sfText_setUnicodeString(text, (sfUint32*) buffer);
		sfRenderWindow_drawText(g->g->render, text, NULL);

		pos.y += 20;
	}

	pos.y += 20;

	for (int i = 0; i < g->u->n_items; i++)
	{
		wchar_t* name = g->u->items[i].name;
		int amount = g->player->inventory.items[i];

		if (amount == 0)
			continue;

		wchar_t buffer[1024];
		swprintf(buffer, 1024, L"%ls: %i", name, amount);

		sfText_setPosition(text, pos);
		sfText_setUnicodeString(text, (sfUint32*) buffer);
		sfRenderWindow_drawText(g->g->render, text, NULL);

		pos.y += 20;
	}
}

char swinventory_catch(swinventory_t* w, game_t* g, float x, float y)
{
	(void) w;
	(void) g;
	(void) x;
	(void) y;
	return 0;
}
