#include "swequipment.h"

void swequipment_init(swequipment_t* w)
{
	subwindow_init(&w->w, L"Equipment", 1024-SW_WIDTH*3, SW_HEIGHT);
}

void swequipment_exit(swequipment_t* w)
{
	subwindow_exit(&w->w);
}

char swequipment_cursor(swequipment_t* w, game_t* g, float x, float y)
{
	(void) w;
	(void) g;
	(void) x;
	(void) y;
	return 0;
}

void swequipment_draw(swequipment_t* w, game_t* g)
{
	if (!subwindow_draw(&w->w, g->g))
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
	for (int i = 0; i < g->u->n_slots; i++)
	{
		sfText_setPosition(text, pos);
		sfText_setUnicodeString(text, (sfUint32*) g->u->slots[i].name);
		sfRenderWindow_drawText(g->g->render, text, NULL);

		int id = g->player->equipment[i];
		sfUint32* txt = (sfUint32*) (id >= 0 ? g->u->items[id].name : L"-");
		pos.x += 150;
		sfText_setPosition(text, pos);
		sfText_setUnicodeString(text, txt);
		sfRenderWindow_drawText(g->g->render, text, NULL);
		pos.x -= 150;

		pos.y += 20;
	}
}

char swequipment_catch(swequipment_t* w, game_t* g, float x, float y, int t)
{
	if (!w->w.visible)
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

	sfVector2f pos = {w->w.x + 20 + 150, w->w.y + 30};
	for (int i = 0; i < g->u->n_slots; i++)
	{
		pos.y += 20;

		int id = g->player->equipment[i];
		sfUint32* txt = (sfUint32*) (id >= 0 ? g->u->items[id].name : L"-");
		sfText_setPosition(text, pos);
		sfText_setUnicodeString(text, txt);

		sfFloatRect rect = sfText_getGlobalBounds(text);
		if (!sfFloatRect_contains(&rect, x, y))
			continue;

		int j = g->player->equipment[i];
		if (j >= 0)
		{
			g->player->equipment[i] = -1;
			g->player->inventory.items[j]++;
		}
		return 1;
	}

	return subwindow_catch(&w->w, g->g, x, y, t);
}
